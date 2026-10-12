use crate::contracts::voice::{AudioOutput, VoiceError};
use crate::voice::audio::AudioCodec;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Trait que abstrai o sink físico ou simulado de reprodução de áudio da Yuki.
pub trait AudioPlaybackSink: Send + Sync + std::fmt::Debug {
    /// Envia o buffer de áudio para reprodução.
    fn play(&self, output: &AudioOutput) -> Result<(), VoiceError>;

    /// Interrompe imediatamente qualquer reprodução física ativa no hardware.
    fn interrupt(&self) -> Result<(), VoiceError>;

    /// Verifica se há dispositivo de saída disponível no sistema operacional.
    fn is_device_available(&self) -> bool;

    /// Nome descritivo da implementação de sink.
    fn name(&self) -> &str;
}

/// Estado atual da reprodução de áudio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Idle,
    Playing,
    Completed,
    Interrupted,
    Stopped,
    Failed,
}

/// Política de autorização para emissão sonora física no subsistema de áudio.
///
/// Invariante de Segurança Acústica:
/// A execução da suíte de testes (`cargo test`) DEVE ser 100% silenciosa por construção,
/// sem depender de inspeção de caminhos de arquivos executáveis (`deps`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioOutputPolicy {
    /// Produção: emite áudio diretamente no hardware via API nativa do sistema operacional (winmm::PlaySoundA).
    Production,
    /// Teste com opt-in explícito: só emite áudio físico no hardware se a variável de ambiente `YUKI_LIVE_AUDIO_TESTS=1`
    /// estiver presente. Sem a variável, valida e armazena o buffer de áudio na memória, mantendo silêncio absoluto.
    OptInTest,
    /// Silenciado por construção: nunca aciona o hardware de áudio; valida o buffer e o mantém na memória.
    Silenced,
}

/// Ambiente operacional para seleção explícita do subsistema de reprodução de áudio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackEnvironment {
    /// Ambiente operacional de produção: conecta diretamente ao backend do sistema operacional (Windows winmm).
    /// Se nenhum dispositivo físico for detectado, falha explicitamente com DeviceUnavailable (sem mock silencioso).
    Production,
    /// Ambiente de testes determinísticos: utiliza MockPlaybackSink silenciado por construção.
    Test,
    /// Reprodução desabilitada: sink silenciado que valida buffers mas nunca emite áudio no hardware.
    Disabled,
}

// ---------------------------------------------------------------------------
// Backend Windows nativo via winmm.dll
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[allow(clippy::upper_case_acronyms)]
mod winmm {
    use std::os::raw::{c_char, c_void};
    pub type BOOL = i32;
    pub type DWORD = u32;
    pub type UINT = u32;
    pub type LPCSTR = *const c_char;
    pub type HMODULE = *mut c_void;

    pub const SND_ASYNC: DWORD = 0x0001;
    pub const SND_NODEFAULT: DWORD = 0x0002;
    pub const SND_MEMORY: DWORD = 0x0004;

    #[link(name = "winmm")]
    extern "system" {
        pub fn PlaySoundA(pszSound: LPCSTR, hmod: HMODULE, fdwSound: DWORD) -> BOOL;
        pub fn waveOutGetNumDevs() -> UINT;
    }
}

/// Verifica se a execução atual possui autorização explícita para emissão de áudio físico em testes.
///
/// Invariante de Segurança Acústica:
/// A execução de testes (`cargo test`) DEVE ser 100% silenciosa por padrão.
/// Emissão física em alto-falantes só é permitida com opt-in explícito via:
/// `YUKI_LIVE_AUDIO_TESTS=1`.
pub fn is_live_audio_test_allowed() -> bool {
    std::env::var("YUKI_LIVE_AUDIO_TESTS").as_deref() == Ok("1")
}

/// Sink concreto para Windows usando APIs nativas multimídia (winmm.dll).
///
/// ARQUITETURA E GERENCIAMENTO DE MEMÓRIA (WinMM Safety Invariants):
/// 1. **Ciclo de Vida de Memória do `PlaySoundA`**:
///    Quando invocado com `SND_MEMORY | SND_ASYNC`, o driver do Windows lê o ponteiro de memória
///    de forma concorrente e assíncrona. O Windows NÃO duplica o buffer internamente na API;
///    ele faz streaming diretamente da memória do processo.
/// 2. **Buffer Pinned em Memória**:
///    Para prevenir corrupção de memória e use-after-free, o `WindowsAudioPlaybackSink` armazena
///    uma referência `Arc<Vec<u8>>` em `current_wav_buffer` durante todo o período em que o som
///    pode estar sendo tocado pelo driver de áudio do kernel.
/// 3. **Interrupção Atômica e Implementação de `Drop`**:
///    Ao substituir um buffer ou destruir a instância (`Drop`), invocamos `PlaySoundA(NULL, NULL, 0)`.
///    Isso força a interrupção imediata da leitura do hardware antes que o ponteiro anterior seja liberado.
/// 4. **Conclusão Estimada vs Confirmação Física**:
///    A API clássica `PlaySoundA` não fornece callbacks assíncronos de finalização (como `waveOutWrite`
///    com eventos de cabeçalho). O `AudioPlaybackController` rastreia o ciclo de vida via temporizador
///    baseado no número de amostras PCM e sample rate (`is_completion_estimated() -> true`).
#[derive(Debug)]
pub struct WindowsAudioPlaybackSink {
    policy: AudioOutputPolicy,
    current_wav_buffer: Arc<Mutex<Option<Arc<Vec<u8>>>>>,
}

impl Drop for WindowsAudioPlaybackSink {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            unsafe {
                winmm::PlaySoundA(std::ptr::null(), std::ptr::null_mut(), 0);
            }
        }
    }
}

impl Default for WindowsAudioPlaybackSink {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsAudioPlaybackSink {
    /// Construtor seguro padrão: utiliza `AudioOutputPolicy::OptInTest`.
    /// Garante que testes que instanciem este sink diretamente não emitam som sem autorização explícita.
    pub fn new() -> Self {
        Self::with_policy(AudioOutputPolicy::OptInTest)
    }

    /// Construtor de produção: emite áudio físico diretamente no hardware sem depender de variáveis de teste.
    pub fn production() -> Self {
        Self::with_policy(AudioOutputPolicy::Production)
    }

    /// Construtor para testes com opt-in: só emite som se `YUKI_LIVE_AUDIO_TESTS=1`.
    pub fn opt_in_test() -> Self {
        Self::with_policy(AudioOutputPolicy::OptInTest)
    }

    /// Construtor estritamente silenciado: nunca emite som no hardware.
    pub fn silenced() -> Self {
        Self::with_policy(AudioOutputPolicy::Silenced)
    }

    /// Construtor parametrizado com a política desejada.
    pub fn with_policy(policy: AudioOutputPolicy) -> Self {
        Self {
            policy,
            current_wav_buffer: Arc::new(Mutex::new(None)),
        }
    }

    /// Retorna a política de saída de áudio ativa.
    pub fn policy(&self) -> AudioOutputPolicy {
        self.policy
    }

    /// Prepara um buffer de áudio WAV válido a partir do payload fornecido.
    #[allow(clippy::chunks_exact_to_as_chunks, clippy::manual_is_multiple_of)]
    fn prepare_wav_bytes(&self, output: &AudioOutput) -> Result<Vec<u8>, VoiceError> {
        if output.payload.starts_with(b"RIFF") && output.payload.len() >= 44 {
            // Já é um contêiner WAV válido
            Ok(output.payload.clone())
        } else if output.payload.len() % 2 == 0 {
            // Trata como PCM 16-bit linear e envelopa em WAV canônico
            let samples: Vec<i16> = output
                .payload
                .chunks_exact(2)
                .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();
            AudioCodec::encode_wav(&samples, &output.format)
        } else {
            Err(VoiceError::InvalidAudioFormat(
                "Buffer de áudio inválido: tamanho ímpar incompatível com PCM 16-bit".to_string(),
            ))
        }
    }
}

impl AudioPlaybackSink for WindowsAudioPlaybackSink {
    fn play(&self, output: &AudioOutput) -> Result<(), VoiceError> {
        #[cfg(windows)]
        {
            let should_emit_physical = match self.policy {
                AudioOutputPolicy::Production => true,
                AudioOutputPolicy::OptInTest => is_live_audio_test_allowed(),
                AudioOutputPolicy::Silenced => false,
            };

            if !should_emit_physical {
                // Em modo não-físico (teste sem opt-in ou silenciado), valida a integridade
                // do buffer WAV e retém o buffer em memória para inspeção, mantendo silêncio absoluto.
                let wav_bytes = Arc::new(self.prepare_wav_bytes(output)?);
                let mut buf_lock = self.current_wav_buffer.lock().unwrap();
                *buf_lock = Some(wav_bytes);
                return Ok(());
            }

            if !self.is_device_available() {
                return Err(VoiceError::DeviceUnavailable(
                    "Nenhum dispositivo de reprodução de áudio detectado no sistema operacional (waveOut)"
                        .to_string(),
                ));
            }

            let wav_bytes = Arc::new(self.prepare_wav_bytes(output)?);

            let mut buf_lock = self.current_wav_buffer.lock().unwrap();
            // Interrompe reprodução prévia no driver antes de registrar novo buffer
            unsafe {
                winmm::PlaySoundA(std::ptr::null(), std::ptr::null_mut(), 0);
            }
            *buf_lock = Some(wav_bytes.clone());

            let ptr = wav_bytes.as_ptr() as winmm::LPCSTR;
            let flags = winmm::SND_MEMORY | winmm::SND_ASYNC | winmm::SND_NODEFAULT;
            let success = unsafe { winmm::PlaySoundA(ptr, std::ptr::null_mut(), flags) };
            if success == 0 {
                *buf_lock = None;
                return Err(VoiceError::PlaybackFailed(
                    "Falha ao acionar winmm::PlaySoundA no subsistema de áudio do Windows"
                        .to_string(),
                ));
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = output;
            Err(VoiceError::PlaybackFailed(
                "WindowsAudioPlaybackSink é suportado apenas em sistemas Windows".to_string(),
            ))
        }
    }

    fn interrupt(&self) -> Result<(), VoiceError> {
        #[cfg(windows)]
        {
            // Interrompe imediatamente qualquer som tocando de forma assíncrona
            unsafe {
                winmm::PlaySoundA(std::ptr::null(), std::ptr::null_mut(), 0);
            }
            let mut buf_lock = self.current_wav_buffer.lock().unwrap();
            *buf_lock = None;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Ok(())
        }
    }

    fn is_device_available(&self) -> bool {
        #[cfg(windows)]
        {
            let devs = unsafe { winmm::waveOutGetNumDevs() };
            devs > 0
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    fn name(&self) -> &str {
        "WindowsAudioPlaybackSink (winmm)"
    }
}

// ---------------------------------------------------------------------------
// Sink de Reprodução Windows WASAPI Core Audio com Seleção de Dispositivo
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Modelo e Classificação de Formato WASAPI (Cross-Platform para Testes)
// ---------------------------------------------------------------------------

#[allow(non_snake_case, dead_code, clippy::upper_case_acronyms)]
pub mod wasapi_format {
    use crate::contracts::voice::VoiceError;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    pub struct WasapiGuid {
        pub data1: u32,
        pub data2: u16,
        pub data3: u16,
        pub data4: [u8; 8],
    }

    pub const KSDATAFORMAT_SUBTYPE_PCM: WasapiGuid = WasapiGuid {
        data1: 0x00000001,
        data2: 0x0000,
        data3: 0x0010,
        data4: [0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71],
    };

    pub const KSDATAFORMAT_SUBTYPE_IEEE_FLOAT: WasapiGuid = WasapiGuid {
        data1: 0x00000003,
        data2: 0x0000,
        data3: 0x0010,
        data4: [0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71],
    };

    pub fn is_ks_guid(guid: &WasapiGuid) -> bool {
        guid.data2 == 0x0000
            && guid.data3 == 0x0010
            && guid.data4 == [0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71]
    }

    #[repr(C, packed(2))]
    #[derive(Clone, Copy, Default)]
    pub struct WAVEFORMATEX {
        pub wFormatTag: u16,
        pub nChannels: u16,
        pub nSamplesPerSec: u32,
        pub nAvgBytesPerSec: u32,
        pub nBlockAlign: u16,
        pub wBitsPerSample: u16,
        pub cbSize: u16,
    }

    #[repr(C, packed(2))]
    #[derive(Clone, Copy, Default)]
    pub struct WAVEFORMATEXTENSIBLE {
        pub format: WAVEFORMATEX,
        pub samples: u16,
        pub dw_channel_mask: u32,
        pub sub_format: WasapiGuid,
    }

    /// Informações descritivas completas de um formato de áudio WASAPI para registro e diagnóstico.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ParsedWaveFormat {
        pub tag: u16,
        pub channels: u16,
        pub sample_rate: u32,
        pub avg_bytes_per_sec: u32,
        pub block_align: u16,
        pub bits_per_sample: u16,
        pub cb_size: u16,
        pub valid_bits_per_sample: Option<u16>,
        pub channel_mask: Option<u32>,
        pub sub_format_data1: Option<u32>,
        pub is_ks_sub_format: bool,
    }

    impl ParsedWaveFormat {
        pub fn summary(&self) -> String {
            let ext_info = if self.tag == 0xFFFE {
                let sub_name = match self.sub_format_data1 {
                    Some(1) if self.is_ks_sub_format => "KSDATAFORMAT_SUBTYPE_PCM (0x01)",
                    Some(3) if self.is_ks_sub_format => "KSDATAFORMAT_SUBTYPE_IEEE_FLOAT (0x03)",
                    Some(_) => "Outro GUID",
                    None => "Nenhum",
                };
                format!(
                    " | Extensible: SubFormat={}, SubFormatData1={:?}, is_ks={}, wValidBitsPerSample={:?}, dwChannelMask=0x{:08X?}",
                    sub_name,
                    self.sub_format_data1,
                    self.is_ks_sub_format,
                    self.valid_bits_per_sample,
                    self.channel_mask
                )
            } else {
                String::new()
            };
            format!(
                "wFormatTag=0x{:04X}, wBitsPerSample={}, nChannels={}, nSamplesPerSec={}Hz, nBlockAlign={}, cbSize={}{}",
                self.tag,
                self.bits_per_sample,
                self.channels,
                self.sample_rate,
                self.block_align,
                self.cb_size,
                ext_info
            )
        }

        /// Interpreta os bytes do formato retornado pela API WASAPI nativa.
        ///
        /// # Safety
        ///
        /// O ponteiro `ptr` deve apontar para uma estrutura `WAVEFORMATEX` válida e alocada pelo Windows.
        pub unsafe fn from_raw_ptr(ptr: *const WAVEFORMATEX) -> Result<Self, VoiceError> {
            if ptr.is_null() {
                return Err(VoiceError::PlaybackFailed(
                    "Ponteiro de formato de áudio WASAPI nulo".to_string(),
                ));
            }
            let fmt = std::ptr::read_unaligned(ptr);
            let tag = fmt.wFormatTag;
            let channels = fmt.nChannels;
            let sample_rate = fmt.nSamplesPerSec;
            let avg_bytes_per_sec = fmt.nAvgBytesPerSec;
            let block_align = fmt.nBlockAlign;
            let bits_per_sample = fmt.wBitsPerSample;
            let cb_size = fmt.cbSize;

            let (valid_bits_per_sample, channel_mask, sub_format_data1, is_ks_sub_format) =
                if tag == 0xFFFE && cb_size >= 22 {
                    let ext_ptr = ptr as *const WAVEFORMATEXTENSIBLE;
                    let ext = std::ptr::read_unaligned(ext_ptr);
                    let sub_format = ext.sub_format;
                    let is_ks = is_ks_guid(&sub_format);
                    (
                        Some(ext.samples),
                        Some(ext.dw_channel_mask),
                        Some(sub_format.data1),
                        is_ks,
                    )
                } else {
                    (None, None, None, false)
                };

            Ok(Self {
                tag,
                channels,
                sample_rate,
                avg_bytes_per_sec,
                block_align,
                bits_per_sample,
                cb_size,
                valid_bits_per_sample,
                channel_mask,
                sub_format_data1,
                is_ks_sub_format,
            })
        }
    }

    /// Classificação formal do formato de amostra WASAPI para conversão exata.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum WasapiSampleClassification {
        /// IEEE 32-bit Float (-1.0f32 a 1.0f32)
        IeeeFloat32,
        /// PCM Inteiro de 32 bits (MSB-justificado; compatível com 32-bit, 24-in-32 e 16-in-32)
        Pcm32Int { valid_bits: u16 },
        /// PCM Inteiro de 24 bits empacotado (3 bytes por amostra)
        Pcm24Packed,
        /// PCM Inteiro de 16 bits (padrão i16 little-endian)
        Pcm16,
    }

    pub fn classify_sample_format(
        parsed: &ParsedWaveFormat,
    ) -> Result<WasapiSampleClassification, VoiceError> {
        let is_float = if parsed.tag == 0xFFFE {
            parsed.is_ks_sub_format && parsed.sub_format_data1 == Some(3)
        } else {
            parsed.tag == 3
        };

        let is_pcm = if parsed.tag == 0xFFFE {
            parsed.is_ks_sub_format && parsed.sub_format_data1 == Some(1)
        } else {
            parsed.tag == 1
        };

        if is_float {
            if parsed.bits_per_sample == 32 {
                Ok(WasapiSampleClassification::IeeeFloat32)
            } else {
                Err(VoiceError::PlaybackFailed(format!(
                    "Formato IEEE Float com {} bits por amostra não suportado (esperado 32 bits). Detalhes: {}",
                    parsed.bits_per_sample,
                    parsed.summary()
                )))
            }
        } else if is_pcm {
            match parsed.bits_per_sample {
                16 => Ok(WasapiSampleClassification::Pcm16),
                24 => {
                    if parsed.block_align == parsed.channels * 3 {
                        Ok(WasapiSampleClassification::Pcm24Packed)
                    } else if parsed.block_align == parsed.channels * 4 {
                        let valid = parsed.valid_bits_per_sample.unwrap_or(24);
                        Ok(WasapiSampleClassification::Pcm32Int { valid_bits: valid })
                    } else {
                        Err(VoiceError::PlaybackFailed(format!(
                            "Alinhamento de bloco inesperado para PCM de 24 bits: {} (canais: {}). Detalhes: {}",
                            parsed.block_align, parsed.channels, parsed.summary()
                        )))
                    }
                }
                32 => {
                    let valid = parsed.valid_bits_per_sample.unwrap_or(32);
                    let valid_clamped = if valid > 0 && valid <= 32 { valid } else { 32 };
                    Ok(WasapiSampleClassification::Pcm32Int {
                        valid_bits: valid_clamped,
                    })
                }
                other => Err(VoiceError::PlaybackFailed(format!(
                    "Formato PCM inteiro com {} bits por amostra não suportado (esperado 16, 24 ou 32). Detalhes: {}",
                    other,
                    parsed.summary()
                ))),
            }
        } else {
            Err(VoiceError::PlaybackFailed(format!(
                "Subtipo de áudio WASAPI não suportado. Detalhes: {}",
                parsed.summary()
            )))
        }
    }

    pub fn convert_samples_to_wasapi(
        samples: &[i16],
        channels: usize,
        classification: WasapiSampleClassification,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        match classification {
            WasapiSampleClassification::IeeeFloat32 => {
                out.reserve(samples.len() * channels * 4);
                for &s in samples {
                    let f = (s as f32) / 32767.0f32;
                    let bytes = f.clamp(-1.0, 1.0).to_le_bytes();
                    for _ in 0..channels {
                        out.extend_from_slice(&bytes);
                    }
                }
            }
            WasapiSampleClassification::Pcm32Int { valid_bits: _ } => {
                out.reserve(samples.len() * channels * 4);
                for &s in samples {
                    let val: i32 = (s as i32) << 16;
                    let bytes = val.to_le_bytes();
                    for _ in 0..channels {
                        out.extend_from_slice(&bytes);
                    }
                }
            }
            WasapiSampleClassification::Pcm24Packed => {
                out.reserve(samples.len() * channels * 3);
                for &s in samples {
                    let val: i32 = (s as i32) << 8;
                    let bytes = [
                        (val & 0xFF) as u8,
                        ((val >> 8) & 0xFF) as u8,
                        ((val >> 16) & 0xFF) as u8,
                    ];
                    for _ in 0..channels {
                        out.extend_from_slice(&bytes);
                    }
                }
            }
            WasapiSampleClassification::Pcm16 => {
                out.reserve(samples.len() * channels * 2);
                for &s in samples {
                    let bytes = s.to_le_bytes();
                    for _ in 0..channels {
                        out.extend_from_slice(&bytes);
                    }
                }
            }
        }
        out
    }
}

pub use wasapi_format::*;

// ---------------------------------------------------------------------------
// Sink de Reprodução Windows WASAPI Core Audio com Seleção de Dispositivo
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[allow(non_snake_case, dead_code, clippy::upper_case_acronyms)]
pub mod wasapi_render_ffi {
    use std::ffi::c_void;

    pub type GUID = crate::voice::playback::wasapi_format::WasapiGuid;
    pub type WAVEFORMATEX = crate::voice::playback::wasapi_format::WAVEFORMATEX;
    pub type WAVEFORMATEXTENSIBLE = crate::voice::playback::wasapi_format::WAVEFORMATEXTENSIBLE;
    pub use crate::voice::playback::wasapi_format::{
        is_ks_guid, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, KSDATAFORMAT_SUBTYPE_PCM,
    };

    pub type HRESULT = i32;

    pub const CLSID_MM_DEVICE_ENUMERATOR: GUID = GUID {
        data1: 0xBCDE0395,
        data2: 0xE52F,
        data3: 0x467C,
        data4: [0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E],
    };

    pub const IID_IMM_DEVICE_ENUMERATOR: GUID = GUID {
        data1: 0xA95664D2,
        data2: 0x9614,
        data3: 0x4F35,
        data4: [0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6],
    };

    pub const IID_IAUDIO_CLIENT: GUID = GUID {
        data1: 0x1CB9AD4C,
        data2: 0xDBFA,
        data3: 0x4C32,
        data4: [0xB1, 0x78, 0xC2, 0xF5, 0x68, 0xA7, 0x03, 0xB2],
    };

    pub const IID_IAUDIO_RENDER_CLIENT: GUID = GUID {
        data1: 0xF294ACFC,
        data2: 0x3146,
        data3: 0x4483,
        data4: [0xA7, 0xBF, 0xAD, 0xDC, 0xA7, 0xC2, 0x60, 0xE2],
    };

    #[link(name = "ole32")]
    extern "system" {
        pub fn CoInitializeEx(pv_reserved: *mut c_void, dw_co_init: u32) -> HRESULT;
        pub fn CoUninitialize();
        pub fn CoCreateInstance(
            rclsid: *const GUID,
            p_unk_outer: *mut c_void,
            dw_cls_context: u32,
            riid: *const GUID,
            ppv: *mut *mut c_void,
        ) -> HRESULT;
        pub fn CoTaskMemFree(pv: *mut c_void);
    }

    #[repr(C)]
    pub struct IMMDeviceEnumeratorVtbl {
        pub QueryInterface:
            unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
        pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
        pub EnumAudioEndpoints:
            unsafe extern "system" fn(*mut c_void, i32, u32, *mut *mut c_void) -> HRESULT,
        pub GetDefaultAudioEndpoint:
            unsafe extern "system" fn(*mut c_void, i32, i32, *mut *mut c_void) -> HRESULT,
        pub GetDevice:
            unsafe extern "system" fn(*mut c_void, *const u16, *mut *mut c_void) -> HRESULT,
    }

    #[repr(C)]
    pub struct IMMDeviceVtbl {
        pub QueryInterface:
            unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
        pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
        pub Activate: unsafe extern "system" fn(
            *mut c_void,
            *const GUID,
            u32,
            *mut c_void,
            *mut *mut c_void,
        ) -> HRESULT,
        pub OpenPropertyStore:
            unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> HRESULT,
        pub GetId: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> HRESULT,
        pub GetState: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
    }

    #[repr(C)]
    pub struct IAudioClientVtbl {
        pub QueryInterface:
            unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
        pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
        pub Initialize: unsafe extern "system" fn(
            *mut c_void,
            i32,
            u32,
            i64,
            i64,
            *const WAVEFORMATEX,
            *const GUID,
        ) -> HRESULT,
        pub GetBufferSize: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
        pub GetStreamLatency: unsafe extern "system" fn(*mut c_void, *mut i64) -> HRESULT,
        pub GetCurrentPadding: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
        pub IsFormatSupported: unsafe extern "system" fn(
            *mut c_void,
            i32,
            *const WAVEFORMATEX,
            *mut *mut WAVEFORMATEX,
        ) -> HRESULT,
        pub GetMixFormat: unsafe extern "system" fn(*mut c_void, *mut *mut WAVEFORMATEX) -> HRESULT,
        pub GetDevicePeriod: unsafe extern "system" fn(*mut c_void, *mut i64, *mut i64) -> HRESULT,
        pub Start: unsafe extern "system" fn(*mut c_void) -> HRESULT,
        pub Stop: unsafe extern "system" fn(*mut c_void) -> HRESULT,
        pub Reset: unsafe extern "system" fn(*mut c_void) -> HRESULT,
        pub SetEventHandle: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT,
        pub GetService:
            unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    }

    #[repr(C)]
    pub struct IAudioRenderClientVtbl {
        pub QueryInterface:
            unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
        pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
        pub GetBuffer: unsafe extern "system" fn(*mut c_void, u32, *mut *mut u8) -> HRESULT,
        pub ReleaseBuffer: unsafe extern "system" fn(*mut c_void, u32, u32) -> HRESULT,
    }
}

/// Sink de áudio nativo para Windows baseado na API oficial WASAPI Core Audio com suporte a seleção de endpoint.
#[derive(Debug)]
pub struct WasapiAudioPlaybackSink {
    device_id: Option<String>,
    policy: AudioOutputPolicy,
    is_playing: Arc<AtomicBool>,
    current_wav_buffer: Arc<Mutex<Option<Arc<Vec<u8>>>>>,
}

impl Default for WasapiAudioPlaybackSink {
    fn default() -> Self {
        Self::new(None)
    }
}

impl WasapiAudioPlaybackSink {
    pub fn new(device_id: Option<String>) -> Self {
        Self::with_policy(device_id, AudioOutputPolicy::OptInTest)
    }

    pub fn production(device_id: Option<String>) -> Self {
        Self::with_policy(device_id, AudioOutputPolicy::Production)
    }

    pub fn opt_in_test(device_id: Option<String>) -> Self {
        Self::with_policy(device_id, AudioOutputPolicy::OptInTest)
    }

    pub fn silenced() -> Self {
        Self::with_policy(None, AudioOutputPolicy::Silenced)
    }

    pub fn with_policy(device_id: Option<String>, policy: AudioOutputPolicy) -> Self {
        Self {
            device_id,
            policy,
            is_playing: Arc::new(AtomicBool::new(false)),
            current_wav_buffer: Arc::new(Mutex::new(None)),
        }
    }

    pub fn device_id(&self) -> Option<&str> {
        self.device_id.as_deref()
    }

    fn resample_linear(samples: &[i16], in_rate: u32, out_rate: u32) -> Vec<i16> {
        if in_rate == out_rate || samples.is_empty() {
            return samples.to_vec();
        }
        let ratio = in_rate as f64 / out_rate as f64;
        let out_len = ((samples.len() as f64) / ratio).round() as usize;
        let mut out = Vec::with_capacity(out_len);

        for i in 0..out_len {
            let in_pos = i as f64 * ratio;
            let idx = in_pos.floor() as usize;
            let frac = in_pos - (idx as f64);

            let s0 = samples.get(idx).copied().unwrap_or(0) as f64;
            let s1 = samples.get(idx + 1).copied().unwrap_or(s0 as i16) as f64;

            let interpolated = (1.0 - frac) * s0 + frac * s1;
            out.push(interpolated.clamp(-32768.0, 32767.0) as i16);
        }
        out
    }
}

impl AudioPlaybackSink for WasapiAudioPlaybackSink {
    fn play(&self, output: &AudioOutput) -> Result<(), VoiceError> {
        #[cfg(windows)]
        {
            use std::time::Duration;
            use wasapi_render_ffi::*;

            let should_emit_physical = match self.policy {
                AudioOutputPolicy::Production => true,
                AudioOutputPolicy::OptInTest => is_live_audio_test_allowed(),
                AudioOutputPolicy::Silenced => false,
            };

            if !should_emit_physical {
                let wav_bytes = Arc::new(output.payload.clone());
                *self.current_wav_buffer.lock().unwrap() = Some(wav_bytes);
                return Ok(());
            }

            if !self.is_device_available() {
                return Err(VoiceError::DeviceUnavailable(
                    "Nenhum dispositivo de reprodução de áudio ativo detectado (WASAPI)"
                        .to_string(),
                ));
            }

            let (format, samples) =
                if output.payload.starts_with(b"RIFF") && output.payload.len() >= 44 {
                    AudioCodec::decode_wav(&output.payload)?
                } else {
                    #[allow(clippy::chunks_exact_to_as_chunks)]
                    let s: Vec<i16> = output
                        .payload
                        .chunks_exact(2)
                        .map(|c| i16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    (output.format.clone(), s)
                };

            unsafe {
                let hr = CoInitializeEx(std::ptr::null_mut(), 0);
                if hr != 0 && hr != 1 {
                    return Err(VoiceError::PlaybackFailed(format!(
                        "Falha ao inicializar COM para reprodução WASAPI (0x{:08X})",
                        hr
                    )));
                }

                let mut p_enum: *mut std::ffi::c_void = std::ptr::null_mut();
                let hr = CoCreateInstance(
                    &CLSID_MM_DEVICE_ENUMERATOR,
                    std::ptr::null_mut(),
                    1 | 2 | 4 | 16,
                    &IID_IMM_DEVICE_ENUMERATOR,
                    &mut p_enum,
                );
                if hr != 0 || p_enum.is_null() {
                    CoUninitialize();
                    return Err(VoiceError::PlaybackFailed(format!(
                        "Falha ao instanciar MMDeviceEnumerator para playback (0x{:08X})",
                        hr
                    )));
                }

                let enum_vtbl = *(p_enum as *mut *const IMMDeviceEnumeratorVtbl);
                let mut p_dev: *mut std::ffi::c_void = std::ptr::null_mut();

                if let Some(ref dev_id) = self.device_id {
                    let wide_id: Vec<u16> =
                        dev_id.encode_utf16().chain(std::iter::once(0)).collect();
                    let hr = ((*enum_vtbl).GetDevice)(p_enum, wide_id.as_ptr(), &mut p_dev);
                    if hr != 0 || p_dev.is_null() {
                        ((*enum_vtbl).Release)(p_enum);
                        CoUninitialize();
                        return Err(VoiceError::DeviceUnavailable(format!(
                            "Dispositivo de saída especificado '{}' não encontrado (0x{:08X})",
                            dev_id, hr
                        )));
                    }
                } else {
                    let hr = ((*enum_vtbl).GetDefaultAudioEndpoint)(
                        p_enum, 0, /* eRender */
                        0, /* eConsole */
                        &mut p_dev,
                    );
                    if hr != 0 || p_dev.is_null() {
                        ((*enum_vtbl).Release)(p_enum);
                        CoUninitialize();
                        return Err(VoiceError::DeviceUnavailable(format!(
                            "Nenhum endpoint padrão de renderização disponível no Windows (0x{:08X})",
                            hr
                        )));
                    }
                }

                let dev_vtbl = *(p_dev as *mut *const IMMDeviceVtbl);
                let mut dev_state = 0u32;
                ((*dev_vtbl).GetState)(p_dev, &mut dev_state);
                if dev_state != 1 {
                    ((*dev_vtbl).Release)(p_dev);
                    ((*enum_vtbl).Release)(p_enum);
                    CoUninitialize();
                    return Err(VoiceError::DeviceUnavailable(format!(
                        "O dispositivo de saída selecionado não está ativo (estado: {})",
                        dev_state
                    )));
                }

                let mut p_client: *mut std::ffi::c_void = std::ptr::null_mut();
                let hr = ((*dev_vtbl).Activate)(
                    p_dev,
                    &IID_IAUDIO_CLIENT,
                    1 | 2 | 4 | 16,
                    std::ptr::null_mut(),
                    &mut p_client,
                );
                if hr != 0 || p_client.is_null() {
                    ((*dev_vtbl).Release)(p_dev);
                    ((*enum_vtbl).Release)(p_enum);
                    CoUninitialize();
                    return Err(VoiceError::PlaybackFailed(format!(
                        "Falha ao ativar IAudioClient no dispositivo de saída (0x{:08X})",
                        hr
                    )));
                }

                let client_vtbl = *(p_client as *mut *const IAudioClientVtbl);
                let mut p_format: *mut WAVEFORMATEX = std::ptr::null_mut();
                let hr = ((*client_vtbl).GetMixFormat)(p_client, &mut p_format);
                if hr != 0 || p_format.is_null() {
                    ((*client_vtbl).Release)(p_client);
                    ((*dev_vtbl).Release)(p_dev);
                    ((*enum_vtbl).Release)(p_enum);
                    CoUninitialize();
                    return Err(VoiceError::PlaybackFailed(format!(
                        "Falha ao obter formato de mixagem do dispositivo de saída (0x{:08X})",
                        hr
                    )));
                }

                let parsed = match ParsedWaveFormat::from_raw_ptr(p_format) {
                    Ok(p) => p,
                    Err(e) => {
                        CoTaskMemFree(p_format as *mut std::ffi::c_void);
                        ((*client_vtbl).Release)(p_client);
                        ((*dev_vtbl).Release)(p_dev);
                        ((*enum_vtbl).Release)(p_enum);
                        CoUninitialize();
                        return Err(e);
                    }
                };

                let classification = match classify_sample_format(&parsed) {
                    Ok(c) => c,
                    Err(e) => {
                        CoTaskMemFree(p_format as *mut std::ffi::c_void);
                        ((*client_vtbl).Release)(p_client);
                        ((*dev_vtbl).Release)(p_dev);
                        ((*enum_vtbl).Release)(p_enum);
                        CoUninitialize();
                        return Err(e);
                    }
                };

                if parsed.block_align == 0 || parsed.channels == 0 || parsed.sample_rate == 0 {
                    CoTaskMemFree(p_format as *mut std::ffi::c_void);
                    ((*client_vtbl).Release)(p_client);
                    ((*dev_vtbl).Release)(p_dev);
                    ((*enum_vtbl).Release)(p_enum);
                    CoUninitialize();
                    return Err(VoiceError::PlaybackFailed(format!(
                        "Parâmetros de formato inválidos retornados pelo dispositivo de saída: {}",
                        parsed.summary()
                    )));
                }

                println!(
                    "[WASAPI Playback] Formato do endpoint negociado via GetMixFormat: {} | Classificação: {:?}",
                    parsed.summary(),
                    classification
                );

                let mono_samples = if format.channels > 1 {
                    AudioCodec::to_mono(&samples, format.channels)
                } else {
                    samples
                };

                let resampled_samples =
                    Self::resample_linear(&mono_samples, format.sample_rate, parsed.sample_rate);
                let target_channels = parsed.channels as usize;

                let converted_bytes =
                    convert_samples_to_wasapi(&resampled_samples, target_channels, classification);

                let hr = ((*client_vtbl).Initialize)(
                    p_client,
                    0,
                    0,
                    5_000_000,
                    0,
                    p_format,
                    std::ptr::null(),
                );
                if hr != 0 {
                    CoTaskMemFree(p_format as *mut std::ffi::c_void);
                    ((*client_vtbl).Release)(p_client);
                    ((*dev_vtbl).Release)(p_dev);
                    ((*enum_vtbl).Release)(p_enum);
                    CoUninitialize();
                    return Err(VoiceError::PlaybackFailed(format!(
                        "Falha ao inicializar IAudioClient para reprodução (0x{:08X})",
                        hr
                    )));
                }

                let mut buffer_size_frames = 0u32;
                ((*client_vtbl).GetBufferSize)(p_client, &mut buffer_size_frames);

                let mut p_render: *mut std::ffi::c_void = std::ptr::null_mut();
                let hr =
                    ((*client_vtbl).GetService)(p_client, &IID_IAUDIO_RENDER_CLIENT, &mut p_render);
                if hr != 0 || p_render.is_null() {
                    CoTaskMemFree(p_format as *mut std::ffi::c_void);
                    ((*client_vtbl).Release)(p_client);
                    ((*dev_vtbl).Release)(p_dev);
                    ((*enum_vtbl).Release)(p_enum);
                    CoUninitialize();
                    return Err(VoiceError::PlaybackFailed(format!(
                        "Falha ao obter IAudioRenderClient (0x{:08X})",
                        hr
                    )));
                }

                let render_vtbl = *(p_render as *mut *const IAudioRenderClientVtbl);
                self.is_playing.store(true, Ordering::SeqCst);

                let block_align = parsed.block_align as usize;
                let total_frames = converted_bytes.len() / block_align;
                let mut frames_written = 0usize;

                let hr = ((*client_vtbl).Start)(p_client);
                if hr != 0 {
                    self.is_playing.store(false, Ordering::SeqCst);
                    ((*render_vtbl).Release)(p_render);
                    CoTaskMemFree(p_format as *mut std::ffi::c_void);
                    ((*client_vtbl).Release)(p_client);
                    ((*dev_vtbl).Release)(p_dev);
                    ((*enum_vtbl).Release)(p_enum);
                    CoUninitialize();
                    return Err(VoiceError::PlaybackFailed(format!(
                        "Falha ao iniciar IAudioClient para reprodução (0x{:08X})",
                        hr
                    )));
                }

                while frames_written < total_frames && self.is_playing.load(Ordering::SeqCst) {
                    let mut padding = 0u32;
                    let _ = ((*client_vtbl).GetCurrentPadding)(p_client, &mut padding);
                    let available = (buffer_size_frames.saturating_sub(padding)) as usize;
                    let to_write = (total_frames - frames_written).min(available);

                    if to_write > 0 {
                        let mut p_data: *mut u8 = std::ptr::null_mut();
                        let hr = ((*render_vtbl).GetBuffer)(p_render, to_write as u32, &mut p_data);
                        if hr == 0 && !p_data.is_null() {
                            let start_byte = frames_written * block_align;
                            let end_byte = start_byte + (to_write * block_align);
                            let slice = &converted_bytes[start_byte..end_byte];
                            std::ptr::copy_nonoverlapping(slice.as_ptr(), p_data, slice.len());
                            let _ = ((*render_vtbl).ReleaseBuffer)(p_render, to_write as u32, 0);
                            frames_written += to_write;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }

                while self.is_playing.load(Ordering::SeqCst) {
                    let mut padding = 0u32;
                    let _ = ((*client_vtbl).GetCurrentPadding)(p_client, &mut padding);
                    if padding == 0 {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }

                let _ = ((*client_vtbl).Stop)(p_client);
                self.is_playing.store(false, Ordering::SeqCst);

                ((*render_vtbl).Release)(p_render);
                CoTaskMemFree(p_format as *mut std::ffi::c_void);
                ((*client_vtbl).Release)(p_client);
                ((*dev_vtbl).Release)(p_dev);
                ((*enum_vtbl).Release)(p_enum);
                CoUninitialize();

                Ok(())
            }
        }
        #[cfg(not(windows))]
        {
            let _ = output;
            Err(VoiceError::PlaybackFailed(
                "WasapiAudioPlaybackSink é suportado apenas no Windows".to_string(),
            ))
        }
    }

    fn interrupt(&self) -> Result<(), VoiceError> {
        self.is_playing.store(false, Ordering::SeqCst);
        let mut buf_lock = self.current_wav_buffer.lock().unwrap();
        *buf_lock = None;
        Ok(())
    }

    fn is_device_available(&self) -> bool {
        #[cfg(windows)]
        {
            use crate::voice::devices::{list_audio_devices, AudioDeviceFlow};
            if let Ok(devs) = list_audio_devices(true) {
                if let Some(ref target_id) = self.device_id {
                    devs.iter().any(|d| {
                        d.flow == AudioDeviceFlow::Output && d.id.eq_ignore_ascii_case(target_id)
                    })
                } else {
                    devs.iter().any(|d| d.flow == AudioDeviceFlow::Output)
                }
            } else {
                false
            }
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    fn name(&self) -> &str {
        "WindowsAudioPlaybackSink (WASAPI Core Audio)"
    }
}

// ---------------------------------------------------------------------------
// Sink Mock Observável para Testes e CI Headless
// ---------------------------------------------------------------------------

/// Sink observável em memória para validação determinística e ambientes CI sem placa de som.
#[derive(Debug)]
pub struct MockPlaybackSink {
    played_outputs: Arc<Mutex<Vec<AudioOutput>>>,
    interrupted_count: Arc<AtomicUsize>,
    device_available: Arc<AtomicBool>,
    interrupt_fails: Arc<AtomicBool>,
}

impl Default for MockPlaybackSink {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPlaybackSink {
    pub fn new() -> Self {
        Self {
            played_outputs: Arc::new(Mutex::new(Vec::new())),
            interrupted_count: Arc::new(AtomicUsize::new(0)),
            device_available: Arc::new(AtomicBool::new(true)),
            interrupt_fails: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_device_available(available: bool) -> Self {
        let sink = Self::new();
        sink.set_device_available(available);
        sink
    }

    pub fn set_device_available(&self, available: bool) {
        self.device_available.store(available, Ordering::SeqCst);
    }

    pub fn set_interrupt_fails(&self, fails: bool) {
        self.interrupt_fails.store(fails, Ordering::SeqCst);
    }

    pub fn is_interrupt_failing(&self) -> bool {
        self.interrupt_fails.load(Ordering::SeqCst)
    }

    pub fn played_count(&self) -> usize {
        self.played_outputs.lock().unwrap().len()
    }

    pub fn last_played(&self) -> Option<AudioOutput> {
        self.played_outputs.lock().unwrap().last().cloned()
    }

    pub fn interrupted_count(&self) -> usize {
        self.interrupted_count.load(Ordering::SeqCst)
    }

    pub fn clear(&self) {
        self.played_outputs.lock().unwrap().clear();
        self.interrupted_count.store(0, Ordering::SeqCst);
        self.interrupt_fails.store(false, Ordering::SeqCst);
    }
}

impl AudioPlaybackSink for MockPlaybackSink {
    fn play(&self, output: &AudioOutput) -> Result<(), VoiceError> {
        if !self.is_device_available() {
            return Err(VoiceError::DeviceUnavailable(
                "Dispositivo de reprodução mock configurado como indisponível".to_string(),
            ));
        }
        self.played_outputs.lock().unwrap().push(output.clone());
        Ok(())
    }

    fn interrupt(&self) -> Result<(), VoiceError> {
        if self.interrupt_fails.load(Ordering::SeqCst) {
            return Err(VoiceError::PlaybackFailed(
                "Falha simulada na interrupção do sink de áudio".to_string(),
            ));
        }
        self.interrupted_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn is_device_available(&self) -> bool {
        self.device_available.load(Ordering::SeqCst)
    }

    fn name(&self) -> &str {
        "MockPlaybackSink"
    }
}

// ---------------------------------------------------------------------------
// AudioPlaybackController
// ---------------------------------------------------------------------------

/// Estado interno encapsulado sob único Mutex para prevenir deadlocks e condições de corrida entre campos.
#[derive(Debug)]
struct PlaybackStateInner {
    status: PlaybackStatus,
    current_output: Option<AudioOutput>,
    device_available_override: Option<bool>,
    playback_timing: Option<(std::time::Instant, u64)>,
}

impl Default for PlaybackStateInner {
    fn default() -> Self {
        Self {
            status: PlaybackStatus::Idle,
            current_output: None,
            device_available_override: None,
            playback_timing: None,
        }
    }
}

/// Controlador seguro de saída e reprodução de áudio da Yuki.
///
/// Garante:
/// - Acoplamento direto com o subsistema de áudio via `AudioPlaybackSink`.
/// - Prevenção estrita de sobreposição acidental de fala.
/// - Interrupção física instantânea na detecção de fala do operador (barge-in).
/// - Propagação rigorosa de falhas em interrupção com transição para `PlaybackStatus::Failed`.
/// - Rejeição segura de novo áudio caso a interrupção da reprodução anterior falhe.
/// - Tratamento explícito de dispositivo de áudio indisponível ou inacessível.
/// - Limpeza imediata de buffers e recursos na finalização ou cancelamento.
/// - Proteção de concorrência com mutex único atômico (`PlaybackStateInner`).
/// - Silêncio absoluto por padrão na suíte de testes (`MockPlaybackSink` por construção).
#[derive(Debug, Clone)]
pub struct AudioPlaybackController {
    state: Arc<Mutex<PlaybackStateInner>>,
    sink: Arc<dyn AudioPlaybackSink>,
}

impl Default for AudioPlaybackController {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlaybackController {
    /// Construtor padrão seguro: inicializa com `MockPlaybackSink`.
    ///
    /// Por construção, qualquer chamada a `new()` ou `default()` em testes ou instâncias
    /// genéricas é 100% silenciosa e não depende de inspeção de caminho de arquivo executável.
    pub fn new() -> Self {
        Self::with_sink(Arc::new(MockPlaybackSink::new()))
    }

    /// Construtor para ambiente operacional de produção: conecta diretamente ao subsistema nativo do sistema operacional.
    ///
    /// No Windows, conecta ao `WasapiAudioPlaybackSink::production(None)` (WASAPI Core Audio).
    /// Não realiza fallback silencioso para mock: se nenhum dispositivo de hardware estiver
    /// presente ou acessível, `is_device_available()` retornará `false` e chamadas a `play()`
    /// falharão explicitamente com `VoiceError::DeviceUnavailable`, preservando a verdade operacional.
    pub fn production() -> Self {
        #[cfg(windows)]
        {
            Self::with_sink(Arc::new(WasapiAudioPlaybackSink::production(None)))
        }
        #[cfg(not(windows))]
        {
            Self::with_sink(Arc::new(MockPlaybackSink::new()))
        }
    }

    /// Construtor para ambiente operacional de produção com seleção opcional de dispositivo.
    ///
    /// No Windows, conecta ao `WasapiAudioPlaybackSink::production(device_id)` direcionado
    /// para o endpoint correspondente (ex: headset Bluetooth, USB DAC ou Realtek). Se omitido,
    /// utiliza o endpoint de renderização padrão do sistema.
    pub fn production_with_device(device_id: Option<String>) -> Self {
        #[cfg(windows)]
        {
            Self::with_sink(Arc::new(WasapiAudioPlaybackSink::production(device_id)))
        }
        #[cfg(not(windows))]
        {
            let _ = device_id;
            Self::with_sink(Arc::new(MockPlaybackSink::new()))
        }
    }

    /// Construtor para o backend legado winmm (PlaySoundA), preservado para diagnóstico e compatibilidade.
    pub fn winmm() -> Self {
        #[cfg(windows)]
        {
            Self::with_sink(Arc::new(WindowsAudioPlaybackSink::production()))
        }
        #[cfg(not(windows))]
        {
            Self::with_sink(Arc::new(MockPlaybackSink::new()))
        }
    }

    /// Construtor baseado no ambiente operacional explicitamente configurado.
    pub fn for_environment(env: PlaybackEnvironment) -> Self {
        match env {
            PlaybackEnvironment::Production => Self::production(),
            PlaybackEnvironment::Test => Self::new(),
            PlaybackEnvironment::Disabled => {
                #[cfg(windows)]
                {
                    Self::with_sink(Arc::new(WasapiAudioPlaybackSink::silenced()))
                }
                #[cfg(not(windows))]
                {
                    Self::with_sink(Arc::new(MockPlaybackSink::new()))
                }
            }
        }
    }

    /// Construtor para testes ao vivo (opt-in explícito via `YUKI_LIVE_AUDIO_TESTS=1`).
    pub fn live_test() -> Self {
        #[cfg(windows)]
        {
            Self::with_sink(Arc::new(WindowsAudioPlaybackSink::opt_in_test()))
        }
        #[cfg(not(windows))]
        {
            Self::with_sink(Arc::new(MockPlaybackSink::new()))
        }
    }

    /// Construtor parametrizado com sink explícito.
    pub fn with_sink(sink: Arc<dyn AudioPlaybackSink>) -> Self {
        Self {
            state: Arc::new(Mutex::new(PlaybackStateInner::default())),
            sink,
        }
    }

    /// Cria uma instância conectada a um `MockPlaybackSink`, retornando o controlador e o mock observável.
    pub fn mock() -> (Self, Arc<MockPlaybackSink>) {
        let mock_sink = Arc::new(MockPlaybackSink::new());
        let controller = Self::with_sink(mock_sink.clone());
        (controller, mock_sink)
    }

    /// Construtor conveniente para testes determinísticos.
    pub fn for_test() -> Self {
        Self::mock().0
    }

    /// Retorna referência ao sink de reprodução ativo.
    pub fn sink(&self) -> &Arc<dyn AudioPlaybackSink> {
        &self.sink
    }

    /// Define o estado de disponibilidade do dispositivo de áudio (para testes e diagnósticos).
    pub fn set_device_available(&self, available: bool) {
        let mut state = self.state.lock().unwrap();
        state.device_available_override = Some(available);
    }

    /// Verifica se há dispositivo de áudio disponível, respeitando sobrescrições manuais.
    pub fn is_device_available(&self) -> bool {
        let state = self.state.lock().unwrap();
        if let Some(forced) = state.device_available_override {
            return forced;
        }
        drop(state);
        self.sink.is_device_available()
    }

    /// Retorna o estado atual da reprodução, atualizando atomicamente para Completed se a duração do áudio tiver expirado.
    pub fn status(&self) -> PlaybackStatus {
        let mut state = self.state.lock().unwrap();
        if state.status == PlaybackStatus::Playing {
            if let Some((start_instant, duration_ms)) = state.playback_timing {
                if start_instant.elapsed().as_millis() as u64 >= duration_ms {
                    state.status = PlaybackStatus::Completed;
                    state.current_output = None;
                }
            }
        }
        state.status
    }

    /// Retorna cópia do áudio atualmente em reprodução, se houver.
    pub fn current_output(&self) -> Option<AudioOutput> {
        self.state.lock().unwrap().current_output.clone()
    }

    /// Declara explicitamente se a transição para `PlaybackStatus::Completed` é calculada com base
    /// na estimativa de duração dos quadros PCM e taxa de amostragem, em vez de callback físico do driver.
    pub fn is_completion_estimated(&self) -> bool {
        true
    }

    /// Envia um buffer de áudio sintetizado para reprodução.
    ///
    /// Invariantes de Segurança e Tratamento de Erro:
    /// - Se o dispositivo estiver indisponível, falha imediatamente com `DeviceUnavailable`.
    /// - Se houver reprodução ativa (`Playing`), tenta interrompê-la antes de iniciar a nova.
    /// - SE A INTERRUPÇÃO PRÉVIA FALHAR:
    ///   - O novo áudio NÃO é reproduzido sobre o áudio em falha.
    ///   - O estado do controlador transita para `PlaybackStatus::Failed`.
    ///   - Os recursos de timing e o buffer atual são limpos.
    ///   - O erro retornado pelo sink é registrado em `tracing::error!` e propagado ao chamador.
    /// - Se o envio para o sink falhar, transita para `Failed`, limpa buffers e retorna o erro.
    pub fn play(&self, output: &AudioOutput) -> Result<(), VoiceError> {
        if !self.is_device_available() {
            let mut state = self.state.lock().unwrap();
            state.status = PlaybackStatus::Failed;
            return Err(VoiceError::DeviceUnavailable(
                "Dispositivo de reprodução de áudio (alto-falante) não detectado ou inacessível"
                    .to_string(),
            ));
        }

        // Interrompe reprodução prévia no sink se já estiver tocando
        if self.status() == PlaybackStatus::Playing {
            if let Err(e) = self.sink.interrupt() {
                tracing::error!(
                    error = %e,
                    "Falha ao interromper reprodução ativa antes de iniciar novo áudio"
                );
                let mut state = self.state.lock().unwrap();
                state.status = PlaybackStatus::Failed;
                state.playback_timing = None;
                state.current_output = None;
                return Err(e);
            }
        }

        // Envia para o sink físico/mock
        if let Err(e) = self.sink.play(output) {
            tracing::error!(
                error = %e,
                "Falha ao enviar buffer para o sink de reprodução de áudio"
            );
            let mut state = self.state.lock().unwrap();
            state.status = PlaybackStatus::Failed;
            state.playback_timing = None;
            state.current_output = None;
            return Err(e);
        }

        let duration_ms = if output.duration_ms > 0 {
            output.duration_ms
        } else if output.format.sample_rate > 0 {
            ((output.payload.len() as u64) * 1000)
                / (output.format.sample_rate as u64 * output.format.channels as u64 * 2)
        } else {
            100
        };

        let mut state = self.state.lock().unwrap();
        state.playback_timing = Some((std::time::Instant::now(), duration_ms));
        state.current_output = Some(output.clone());
        state.status = PlaybackStatus::Playing;

        Ok(())
    }

    /// Interrompe imediatamente a reprodução em andamento (ex: barge-in do usuário).
    ///
    /// Se a interrupção no sink falhar:
    /// - O estado transita para `PlaybackStatus::Failed`.
    /// - Os recursos de timing e buffers são limpos.
    /// - O erro é registrado em `tracing::error!` e retornado formalmente.
    pub fn interrupt(&self) -> Result<(), VoiceError> {
        if let Err(e) = self.sink.interrupt() {
            tracing::error!(
                error = %e,
                "Falha ao interromper sink de reprodução de áudio"
            );
            let mut state = self.state.lock().unwrap();
            state.status = PlaybackStatus::Failed;
            state.playback_timing = None;
            state.current_output = None;
            return Err(e);
        }

        let mut state = self.state.lock().unwrap();
        state.status = PlaybackStatus::Interrupted;
        state.playback_timing = None;
        state.current_output = None;

        Ok(())
    }

    /// Encerra a reprodução e coloca o controlador no estado Stopped.
    ///
    /// Propaga falhas do sink de áudio, transitando para `PlaybackStatus::Failed` em caso de erro.
    pub fn stop(&self) -> Result<(), VoiceError> {
        if let Err(e) = self.sink.interrupt() {
            tracing::error!(
                error = %e,
                "Falha ao interromper sink de reprodução durante stop()"
            );
            let mut state = self.state.lock().unwrap();
            state.status = PlaybackStatus::Failed;
            state.playback_timing = None;
            state.current_output = None;
            return Err(e);
        }

        let mut state = self.state.lock().unwrap();
        state.status = PlaybackStatus::Stopped;
        state.playback_timing = None;
        state.current_output = None;

        Ok(())
    }

    /// Reseta o controlador para o estado inicial Idle.
    ///
    /// Propaga falhas do sink de áudio, transitando para `PlaybackStatus::Failed` em caso de erro.
    pub fn reset(&self) -> Result<(), VoiceError> {
        if let Err(e) = self.sink.interrupt() {
            tracing::error!(
                error = %e,
                "Falha ao interromper sink de reprodução durante reset()"
            );
            let mut state = self.state.lock().unwrap();
            state.status = PlaybackStatus::Failed;
            state.playback_timing = None;
            state.current_output = None;
            return Err(e);
        }

        let mut state = self.state.lock().unwrap();
        state.status = PlaybackStatus::Idle;
        state.playback_timing = None;
        state.current_output = None;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::voice::AudioFormat;

    #[test]
    fn test_mock_playback_sink_lifecycle_and_interrupt_failure() {
        let sink = MockPlaybackSink::new();
        assert!(sink.is_device_available());
        assert_eq!(sink.played_count(), 0);
        assert!(!sink.is_interrupt_failing());

        let output = AudioOutput::new(vec![0u8; 32], AudioFormat::default(), 1, "test");
        assert!(sink.play(&output).is_ok());
        assert_eq!(sink.played_count(), 1);

        assert!(sink.interrupt().is_ok());
        assert_eq!(sink.interrupted_count(), 1);

        // Simula falha na interrupção do sink
        sink.set_interrupt_fails(true);
        assert!(sink.is_interrupt_failing());
        let int_err = sink.interrupt();
        assert!(int_err.is_err());
        assert!(matches!(
            int_err.unwrap_err(),
            VoiceError::PlaybackFailed(_)
        ));
        assert_eq!(sink.interrupted_count(), 1); // Não incrementou por conta da falha

        // Recupera sucesso
        sink.set_interrupt_fails(false);
        assert!(sink.interrupt().is_ok());
        assert_eq!(sink.interrupted_count(), 2);

        sink.set_device_available(false);
        assert!(!sink.is_device_available());
        assert!(matches!(
            sink.play(&output),
            Err(VoiceError::DeviceUnavailable(_))
        ));
    }

    #[test]
    fn test_playback_controller_mock_mode() {
        let (controller, mock) = AudioPlaybackController::mock();
        assert_eq!(controller.status(), PlaybackStatus::Idle);

        let output = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 50, "test");
        assert!(controller.play(&output).is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Playing);
        assert_eq!(mock.played_count(), 1);

        assert!(controller.interrupt().is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Interrupted);
        assert_eq!(mock.interrupted_count(), 1);
    }

    #[test]
    fn test_playback_controller_interrupt_error_propagation_and_cleanup() {
        let (controller, mock) = AudioPlaybackController::mock();
        let output = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 1000, "test-err");

        assert!(controller.play(&output).is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Playing);
        assert!(controller.current_output().is_some());

        // Força falha na interrupção
        mock.set_interrupt_fails(true);
        let int_res = controller.interrupt();
        assert!(int_res.is_err());
        assert!(matches!(
            int_res.unwrap_err(),
            VoiceError::PlaybackFailed(_)
        ));

        // Invariante: estado transiciona para Failed e buffers são limpos
        assert_eq!(controller.status(), PlaybackStatus::Failed);
        assert_eq!(controller.current_output(), None);
    }

    #[test]
    fn test_playback_controller_play_over_playing_fails_if_interrupt_fails() {
        let (controller, mock) = AudioPlaybackController::mock();
        let output1 = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 5000, "audio-1");
        let output2 = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 5000, "audio-2");

        // Inicia áudio 1
        assert!(controller.play(&output1).is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Playing);
        assert_eq!(mock.played_count(), 1);

        // Simula que o sink de áudio falha ao tentar interromper o áudio 1
        mock.set_interrupt_fails(true);

        // Tentar tocar áudio 2 DEVE falhar imediatamente sem tocar o segundo áudio
        let play2_res = controller.play(&output2);
        assert!(play2_res.is_err());
        assert!(matches!(
            play2_res.unwrap_err(),
            VoiceError::PlaybackFailed(_)
        ));

        // Áudio 2 não foi enviado ao sink
        assert_eq!(mock.played_count(), 1);
        assert_eq!(controller.status(), PlaybackStatus::Failed);
        assert_eq!(controller.current_output(), None);
    }

    #[test]
    fn test_playback_controller_stop_and_reset_error_propagation() {
        let (controller, mock) = AudioPlaybackController::mock();
        let output = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 1000, "test-stop");

        assert!(controller.play(&output).is_ok());
        mock.set_interrupt_fails(true);

        // Stop com falha
        let stop_err = controller.stop();
        assert!(stop_err.is_err());
        assert_eq!(controller.status(), PlaybackStatus::Failed);
        assert_eq!(controller.current_output(), None);

        // Reset com falha
        let reset_err = controller.reset();
        assert!(reset_err.is_err());
        assert_eq!(controller.status(), PlaybackStatus::Failed);

        // Com falha desativada, stop e reset retornam Ok
        mock.set_interrupt_fails(false);
        assert!(controller.stop().is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Stopped);
        assert!(controller.reset().is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Idle);
    }

    #[test]
    fn test_playback_controller_silent_by_construction_default() {
        // Construtor padrão inicializa MockPlaybackSink por construção
        let controller = AudioPlaybackController::new();
        assert_eq!(controller.sink().name(), "MockPlaybackSink");
        assert_eq!(controller.status(), PlaybackStatus::Idle);

        let for_test_ctrl = AudioPlaybackController::for_test();
        assert_eq!(for_test_ctrl.sink().name(), "MockPlaybackSink");
    }

    #[test]
    fn test_playback_controller_for_environment_selection() {
        // 1. Ambiente Test seleciona MockPlaybackSink
        let test_ctrl = AudioPlaybackController::for_environment(PlaybackEnvironment::Test);
        assert_eq!(test_ctrl.sink().name(), "MockPlaybackSink");

        // 2. Ambiente Production seleciona WASAPI Core Audio diretamente sem mascarar em mock
        let prod_ctrl = AudioPlaybackController::for_environment(PlaybackEnvironment::Production);
        assert_eq!(
            prod_ctrl.sink().name(),
            "WindowsAudioPlaybackSink (WASAPI Core Audio)"
        );

        // 3. AudioPlaybackController::production() também seleciona diretamente WASAPI Core Audio
        let direct_prod = AudioPlaybackController::production();
        assert_eq!(
            direct_prod.sink().name(),
            "WindowsAudioPlaybackSink (WASAPI Core Audio)"
        );

        // 4. Se o dispositivo estiver indisponível no ambiente de produção, play() falha explicitamente
        // sem mascarar o resultado como sucesso em mock
        direct_prod.set_device_available(false);
        assert!(!direct_prod.is_device_available());
        let output = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 50, "test-env");
        let play_err = direct_prod.play(&output);
        assert!(play_err.is_err());
        assert!(matches!(
            play_err.unwrap_err(),
            VoiceError::DeviceUnavailable(_)
        ));

        // 5. Ambiente Disabled utiliza sink silenciado
        let disabled_ctrl = AudioPlaybackController::for_environment(PlaybackEnvironment::Disabled);
        assert_eq!(
            disabled_ctrl.sink().name(),
            "WindowsAudioPlaybackSink (WASAPI Core Audio)"
        );

        // 6. AudioPlaybackController::winmm() seleciona o sink legado winmm
        let winmm_ctrl = AudioPlaybackController::winmm();
        assert_eq!(winmm_ctrl.sink().name(), "WindowsAudioPlaybackSink (winmm)");
    }

    #[test]
    fn test_playback_controller_completion_estimation_and_output() {
        let (controller, _) = AudioPlaybackController::mock();
        assert!(controller.is_completion_estimated());
        assert_eq!(controller.current_output(), None);

        let output = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 50, "test");
        assert!(controller.play(&output).is_ok());
        assert_eq!(controller.current_output(), Some(output));
        assert!(controller.stop().is_ok());
        assert_eq!(controller.current_output(), None);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_sink_device_detection_and_safety_policies() {
        let sink = WindowsAudioPlaybackSink::opt_in_test();
        assert_eq!(sink.policy(), AudioOutputPolicy::OptInTest);
        let available = sink.is_device_available();
        println!("Windows audio sink device available: {}", available);

        // Sem a variável YUKI_LIVE_AUDIO_TESTS=1, a execução em teste é silenciada com segurança
        let output = AudioOutput::new(vec![0u8; 320], AudioFormat::default(), 10, "test-safety");
        let play_res = sink.play(&output);
        assert!(play_res.is_ok(), "Deve validar buffer sem erro");
        assert!(sink.interrupt().is_ok());

        // Sink silenciado por construção
        let silenced_sink = WindowsAudioPlaybackSink::silenced();
        assert_eq!(silenced_sink.policy(), AudioOutputPolicy::Silenced);
        assert!(silenced_sink.play(&output).is_ok());
        assert!(silenced_sink.interrupt().is_ok());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "Requer autorização física explícita no hardware de áudio via YUKI_LIVE_AUDIO_TESTS=1"]
    fn test_windows_sink_live_opt_in_physical_playback() {
        if !is_live_audio_test_allowed() {
            println!("Pulando teste físico ao vivo: YUKI_LIVE_AUDIO_TESTS não configurada");
            return;
        }

        let sink = WindowsAudioPlaybackSink::production();
        if !sink.is_device_available() {
            println!("Nenhum dispositivo de áudio disponível para teste ao vivo");
            return;
        }

        let tone_samples: Vec<i16> = (0..8000)
            .map(|i| {
                let freq = 440.0;
                let sample_rate = 16000.0;
                ((i as f32 * freq * 2.0 * std::f32::consts::PI / sample_rate).sin() * 8000.0) as i16
            })
            .collect();
        let wav_data = AudioCodec::encode_wav(&tone_samples, &AudioFormat::default()).unwrap();
        let output = AudioOutput::new(wav_data, AudioFormat::default(), 500, "tone-live");

        assert!(sink.play(&output).is_ok());
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(sink.interrupt().is_ok());
    }

    #[test]
    fn test_wasapi_waveformat_layout_and_offsets() {
        use crate::voice::playback::wasapi_format::{WAVEFORMATEX, WAVEFORMATEXTENSIBLE};

        assert_eq!(
            std::mem::size_of::<WAVEFORMATEX>(),
            18,
            "WAVEFORMATEX deve ter exatamente 18 bytes no padrão Windows C ABI"
        );
        assert_eq!(
            std::mem::size_of::<WAVEFORMATEXTENSIBLE>(),
            40,
            "WAVEFORMATEXTENSIBLE deve ter exatamente 40 bytes no padrão Windows C ABI"
        );
        assert_eq!(
            std::mem::align_of::<WAVEFORMATEX>(),
            2,
            "Alinhamento de WAVEFORMATEX deve ser 2 bytes"
        );
        assert_eq!(
            std::mem::align_of::<WAVEFORMATEXTENSIBLE>(),
            2,
            "Alinhamento de WAVEFORMATEXTENSIBLE deve ser 2 bytes"
        );

        // Validação de offsets em WAVEFORMATEXTENSIBLE
        let ext = std::mem::MaybeUninit::<WAVEFORMATEXTENSIBLE>::uninit();
        let base_ptr = ext.as_ptr() as usize;
        unsafe {
            let p_ext = ext.as_ptr();
            let p_samples = std::ptr::addr_of!((*p_ext).samples) as usize;
            let p_mask = std::ptr::addr_of!((*p_ext).dw_channel_mask) as usize;
            let p_sub = std::ptr::addr_of!((*p_ext).sub_format) as usize;

            assert_eq!(
                p_samples - base_ptr,
                18,
                "Offset de 'samples' deve ser 18 bytes"
            );
            assert_eq!(
                p_mask - base_ptr,
                20,
                "Offset de 'dw_channel_mask' deve ser 20 bytes"
            );
            assert_eq!(
                p_sub - base_ptr,
                24,
                "Offset de 'sub_format' (GUID) deve ser exatamente 24 bytes"
            );
        }
    }

    #[test]
    fn test_wasapi_classify_sample_format() {
        use crate::voice::playback::wasapi_format::{
            classify_sample_format, ParsedWaveFormat, WasapiSampleClassification,
        };

        // 1. Extensible IEEE Float (32 bits)
        let ext_float = ParsedWaveFormat {
            tag: 0xFFFE,
            channels: 2,
            sample_rate: 48000,
            avg_bytes_per_sec: 48000 * 8,
            block_align: 8,
            bits_per_sample: 32,
            cb_size: 22,
            valid_bits_per_sample: Some(32),
            channel_mask: Some(3),
            sub_format_data1: Some(3),
            is_ks_sub_format: true,
        };
        assert_eq!(
            classify_sample_format(&ext_float).unwrap(),
            WasapiSampleClassification::IeeeFloat32
        );

        // 2. Extensible 32-bit PCM inteiro
        let ext_pcm32 = ParsedWaveFormat {
            tag: 0xFFFE,
            channels: 2,
            sample_rate: 48000,
            avg_bytes_per_sec: 48000 * 8,
            block_align: 8,
            bits_per_sample: 32,
            cb_size: 22,
            valid_bits_per_sample: Some(32),
            channel_mask: Some(3),
            sub_format_data1: Some(1),
            is_ks_sub_format: true,
        };
        assert_eq!(
            classify_sample_format(&ext_pcm32).unwrap(),
            WasapiSampleClassification::Pcm32Int { valid_bits: 32 }
        );

        // 3. Extensible 24 bits em container de 32 bits
        let ext_pcm24_in_32 = ParsedWaveFormat {
            tag: 0xFFFE,
            channels: 2,
            sample_rate: 48000,
            avg_bytes_per_sec: 48000 * 8,
            block_align: 8,
            bits_per_sample: 32,
            cb_size: 22,
            valid_bits_per_sample: Some(24),
            channel_mask: Some(3),
            sub_format_data1: Some(1),
            is_ks_sub_format: true,
        };
        assert_eq!(
            classify_sample_format(&ext_pcm24_in_32).unwrap(),
            WasapiSampleClassification::Pcm32Int { valid_bits: 24 }
        );

        // 4. Extensible 24 bits empacotado (3 bytes/amostra)
        let ext_pcm24_packed = ParsedWaveFormat {
            tag: 0xFFFE,
            channels: 2,
            sample_rate: 48000,
            avg_bytes_per_sec: 48000 * 6,
            block_align: 6,
            bits_per_sample: 24,
            cb_size: 22,
            valid_bits_per_sample: Some(24),
            channel_mask: Some(3),
            sub_format_data1: Some(1),
            is_ks_sub_format: true,
        };
        assert_eq!(
            classify_sample_format(&ext_pcm24_packed).unwrap(),
            WasapiSampleClassification::Pcm24Packed
        );

        // 5. Extensible 16 bits PCM
        let ext_pcm16 = ParsedWaveFormat {
            tag: 0xFFFE,
            channels: 2,
            sample_rate: 48000,
            avg_bytes_per_sec: 48000 * 4,
            block_align: 4,
            bits_per_sample: 16,
            cb_size: 22,
            valid_bits_per_sample: Some(16),
            channel_mask: Some(3),
            sub_format_data1: Some(1),
            is_ks_sub_format: true,
        };
        assert_eq!(
            classify_sample_format(&ext_pcm16).unwrap(),
            WasapiSampleClassification::Pcm16
        );

        // 6. Legado PCM (tag 1)
        let leg_pcm = ParsedWaveFormat {
            tag: 1,
            channels: 2,
            sample_rate: 44100,
            avg_bytes_per_sec: 44100 * 4,
            block_align: 4,
            bits_per_sample: 16,
            cb_size: 0,
            valid_bits_per_sample: None,
            channel_mask: None,
            sub_format_data1: None,
            is_ks_sub_format: false,
        };
        assert_eq!(
            classify_sample_format(&leg_pcm).unwrap(),
            WasapiSampleClassification::Pcm16
        );

        // 7. Legado IEEE Float (tag 3)
        let leg_float = ParsedWaveFormat {
            tag: 3,
            channels: 2,
            sample_rate: 48000,
            avg_bytes_per_sec: 48000 * 8,
            block_align: 8,
            bits_per_sample: 32,
            cb_size: 0,
            valid_bits_per_sample: None,
            channel_mask: None,
            sub_format_data1: None,
            is_ks_sub_format: false,
        };
        assert_eq!(
            classify_sample_format(&leg_float).unwrap(),
            WasapiSampleClassification::IeeeFloat32
        );

        // 8. Formatos não suportados falham fechados
        let unsupp_tag = ParsedWaveFormat {
            tag: 0x00FF,
            channels: 2,
            sample_rate: 44100,
            avg_bytes_per_sec: 44100 * 4,
            block_align: 4,
            bits_per_sample: 16,
            cb_size: 0,
            valid_bits_per_sample: None,
            channel_mask: None,
            sub_format_data1: None,
            is_ks_sub_format: false,
        };
        assert!(classify_sample_format(&unsupp_tag).is_err());
    }

    #[test]
    fn test_wasapi_convert_samples_all_formats() {
        use crate::voice::playback::wasapi_format::{
            convert_samples_to_wasapi, WasapiSampleClassification,
        };

        let samples: [i16; 4] = [0, 32767, -32768, 16384];

        // 1. IEEE 32-bit Float estéreo (2 canais)
        let float_bytes =
            convert_samples_to_wasapi(&samples, 2, WasapiSampleClassification::IeeeFloat32);
        assert_eq!(
            float_bytes.len(),
            4 * 2 * 4,
            "4 amostras x 2 canais x 4 bytes = 32 bytes"
        );
        let f0_left = f32::from_le_bytes(float_bytes[0..4].try_into().unwrap());
        let f0_right = f32::from_le_bytes(float_bytes[4..8].try_into().unwrap());
        assert_eq!(f0_left, 0.0);
        assert_eq!(f0_right, 0.0);

        let f1_left = f32::from_le_bytes(float_bytes[8..12].try_into().unwrap());
        assert!((f1_left - 1.0).abs() < 1e-4);

        let f2_left = f32::from_le_bytes(float_bytes[16..20].try_into().unwrap());
        assert_eq!(f2_left, -1.0);

        // 2. PCM 32-bit Inteiro estéreo
        let pcm32_bytes = convert_samples_to_wasapi(
            &samples,
            2,
            WasapiSampleClassification::Pcm32Int { valid_bits: 32 },
        );
        assert_eq!(pcm32_bytes.len(), 4 * 2 * 4);
        let i0 = i32::from_le_bytes(pcm32_bytes[0..4].try_into().unwrap());
        assert_eq!(i0, 0);

        let i1 = i32::from_le_bytes(pcm32_bytes[8..12].try_into().unwrap());
        assert_eq!(i1, (32767i32) << 16);

        let i2 = i32::from_le_bytes(pcm32_bytes[16..20].try_into().unwrap());
        assert_eq!(i2, (-32768i32) << 16);

        // 3. PCM 24-bit Empacotado estéreo
        let pcm24_bytes =
            convert_samples_to_wasapi(&samples, 2, WasapiSampleClassification::Pcm24Packed);
        assert_eq!(
            pcm24_bytes.len(),
            4 * 2 * 3,
            "4 amostras x 2 canais x 3 bytes = 24 bytes"
        );
        let sample1_left = [pcm24_bytes[6], pcm24_bytes[7], pcm24_bytes[8]];
        let expected_val: i32 = (32767i32) << 8;
        let expected_bytes = &expected_val.to_le_bytes()[..3];
        assert_eq!(sample1_left, expected_bytes);

        // 4. PCM 16-bit Inteiro estéreo
        let pcm16_bytes = convert_samples_to_wasapi(&samples, 2, WasapiSampleClassification::Pcm16);
        assert_eq!(
            pcm16_bytes.len(),
            4 * 2 * 2,
            "4 amostras x 2 canais x 2 bytes = 16 bytes"
        );
        let s0 = i16::from_le_bytes(pcm16_bytes[0..2].try_into().unwrap());
        let s1 = i16::from_le_bytes(pcm16_bytes[4..6].try_into().unwrap());
        assert_eq!(s0, 0);
        assert_eq!(s1, 32767);
    }

    #[test]
    fn test_wasapi_regression_32bit_output_device_error() {
        use crate::voice::playback::wasapi_format::{
            classify_sample_format, convert_samples_to_wasapi, ParsedWaveFormat,
            WAVEFORMATEXTENSIBLE,
        };

        // Simula o formato exato retornado por GetMixFormat para fones Bluetooth (ex: Redmi Buds 6 Play)
        // Cenário A: Dispositivo com subformato PCM de 32 bits (ou 24-in-32)
        let raw_ext_pcm = WAVEFORMATEXTENSIBLE {
            format: crate::voice::playback::wasapi_format::WAVEFORMATEX {
                wFormatTag: 0xFFFE,
                nChannels: 2,
                nSamplesPerSec: 48000,
                nAvgBytesPerSec: 48000 * 8,
                nBlockAlign: 8,
                wBitsPerSample: 32,
                cbSize: 22,
            },
            samples: 24, // 24 bits válidos em container de 32 bits
            dw_channel_mask: 0x00000003,
            sub_format: crate::voice::playback::wasapi_format::KSDATAFORMAT_SUBTYPE_PCM,
        };

        let parsed_pcm = unsafe { ParsedWaveFormat::from_raw_ptr(&raw_ext_pcm.format as *const _) }
            .expect("Deve decodificar");
        let summary_pcm = parsed_pcm.summary();
        println!("Formato simulado PCM 32-bit: {}", summary_pcm);

        // Verifica que todos os campos obrigatórios estão presentes no registro
        assert!(summary_pcm.contains("wFormatTag=0xFFFE"));
        assert!(summary_pcm.contains("wBitsPerSample=32"));
        assert!(summary_pcm.contains("nChannels=2"));
        assert!(summary_pcm.contains("nSamplesPerSec=48000Hz"));
        assert!(summary_pcm.contains("nBlockAlign=8"));
        assert!(summary_pcm.contains("cbSize=22"));
        assert!(summary_pcm.contains("SubFormat="));
        assert!(summary_pcm.contains("wValidBitsPerSample=Some(24)"));

        // O erro anterior era: "Formato do dispositivo de saída não suportado: 32 bits por amostra"
        // Agora deve classificar com sucesso como Pcm32Int { valid_bits: 24 }
        let class_pcm = classify_sample_format(&parsed_pcm).expect("32-bit PCM NÃO deve falhar");
        assert_eq!(
            class_pcm,
            crate::voice::playback::wasapi_format::WasapiSampleClassification::Pcm32Int {
                valid_bits: 24
            }
        );

        // Cenário B: Dispositivo com subformato IEEE Float de 32 bits
        let mut raw_ext_float = raw_ext_pcm;
        raw_ext_float.samples = 32;
        raw_ext_float.sub_format =
            crate::voice::playback::wasapi_format::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;

        let parsed_float =
            unsafe { ParsedWaveFormat::from_raw_ptr(&raw_ext_float.format as *const _) }
                .expect("Deve decodificar float");
        let summary_float = parsed_float.summary();
        println!("Formato simulado Float 32-bit: {}", summary_float);

        assert!(summary_float.contains("wBitsPerSample=32"));
        assert!(summary_float.contains("KSDATAFORMAT_SUBTYPE_IEEE_FLOAT"));

        let class_float =
            classify_sample_format(&parsed_float).expect("32-bit Float NÃO deve falhar");
        assert_eq!(
            class_float,
            crate::voice::playback::wasapi_format::WasapiSampleClassification::IeeeFloat32
        );

        // Conversão de amostras de fala para o formato negociado
        let speech_samples = vec![1000i16, -2000i16, 3000i16, -4000i16];
        let pcm_bytes = convert_samples_to_wasapi(&speech_samples, 2, class_pcm);
        assert_eq!(pcm_bytes.len(), 4 * 2 * 4);

        let float_bytes = convert_samples_to_wasapi(&speech_samples, 2, class_float);
        assert_eq!(float_bytes.len(), 4 * 2 * 4);
    }
}
