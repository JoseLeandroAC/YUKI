use crate::contracts::identifiers::generate_unique_id;
use crate::contracts::voice::{
    AudioFormat, AudioInput, BufferMetrics, CaptureTimingMetrics, NativeAudioFormatInfo, VoiceError,
};
use crate::voice::audio::AudioCodec;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Trait abstrata para captura de áudio a partir do microfone físico ou simulado.
pub trait AudioCaptureBackend: Send + Sync + std::fmt::Debug {
    /// Inicia a captura de áudio no backend.
    fn start_capture(&self) -> Result<(), VoiceError>;

    /// Conclui a captura de áudio e retorna os dados capturados como `AudioInput` canônico.
    fn stop_capture(&self) -> Result<AudioInput, VoiceError>;

    /// Cancela a captura em andamento, descartando quaisquer amostras pendentes.
    fn cancel_capture(&self) -> Result<(), VoiceError>;

    /// Verifica se há dispositivo de captura (microfone) disponível e acessível no sistema.
    fn is_device_available(&self) -> bool;

    /// Nome identificador da implementação de captura.
    fn name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Backend Windows Nativo via WASAPI / Core Audio (Produção Primária)
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[allow(non_snake_case, dead_code, clippy::upper_case_acronyms)]
mod wasapi_ffi {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct GUID {
        pub data1: u32,
        pub data2: u16,
        pub data3: u16,
        pub data4: [u8; 8],
    }

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

    pub const IID_IAUDIO_CAPTURE_CLIENT: GUID = GUID {
        data1: 0xC8ADBD64,
        data2: 0xE71E,
        data3: 0x48A0,
        data4: [0xA4, 0xDE, 0x18, 0x5C, 0x39, 0x5C, 0xD3, 0x17],
    };

    pub const KSDATAFORMAT_SUBTYPE_IEEE_FLOAT: GUID = GUID {
        data1: 0x00000003,
        data2: 0x0000,
        data3: 0x0010,
        data4: [0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71],
    };

    pub const KSDATAFORMAT_SUBTYPE_PCM: GUID = GUID {
        data1: 0x00000001,
        data2: 0x0000,
        data3: 0x0010,
        data4: [0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71],
    };

    #[repr(C, packed(2))]
    #[derive(Clone, Copy, Debug)]
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
    #[derive(Clone, Copy, Debug)]
    pub struct WAVEFORMATEXTENSIBLE {
        pub format: WAVEFORMATEX,
        pub samples: u16,
        pub dw_channel_mask: u32,
        pub sub_format: GUID,
    }

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
    pub struct IAudioCaptureClientVtbl {
        pub QueryInterface:
            unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
        pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
        pub GetBuffer: unsafe extern "system" fn(
            *mut c_void,
            *mut *mut u8,
            *mut u32,
            *mut u32,
            *mut u64,
            *mut u64,
        ) -> HRESULT,
        pub ReleaseBuffer: unsafe extern "system" fn(*mut c_void, u32) -> HRESULT,
        pub GetNextPacketSize: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
    }
}

/// Backend nativo de captura de áudio para Windows baseado na API oficial WASAPI (Windows Audio Session API).
///
/// Vantagens arquiteturais sobre MCI legado:
/// 1. Suporte nativo a seleção determinística de endpoints por ID estável (USB, Realtek, Bluetooth HFP).
/// 2. Operação estritamente em memória: zero gravação de arquivos temporários em disco.
/// 3. Conversão e downmix transparente de formatos modernos (IEEE 32-bit Float, Stereo 48kHz) para o formato
///    canônico da Yuki (Mono 16kHz PCM 16-bit).
/// 4. Compatibilidade integral com headsets Bluetooth (ex: Redmi Buds 6 Play) via perfil Hands-Free (HFP).
type CaptureWorkerHandle =
    Arc<Mutex<Option<std::thread::JoinHandle<Result<AudioInput, VoiceError>>>>>;

#[derive(Debug)]
pub struct WasapiAudioCaptureBackend {
    device_id: Option<String>,
    is_recording: Arc<AtomicBool>,
    diagnostics_enabled: bool,
    worker_handle: CaptureWorkerHandle,
}

impl Default for WasapiAudioCaptureBackend {
    fn default() -> Self {
        Self::new(None)
    }
}

impl WasapiAudioCaptureBackend {
    pub fn new(device_id: Option<String>) -> Self {
        Self::with_options(device_id, false)
    }

    pub fn with_diagnostics(device_id: Option<String>, diagnostics: bool) -> Self {
        Self::with_options(device_id, diagnostics)
    }

    pub fn with_options(device_id: Option<String>, diagnostics: bool) -> Self {
        Self {
            device_id,
            is_recording: Arc::new(AtomicBool::new(false)),
            diagnostics_enabled: diagnostics,
            worker_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Resample linear de uma sequência mono de amostras `i16` para a taxa de amostragem alvo.
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

impl AudioCaptureBackend for WasapiAudioCaptureBackend {
    fn start_capture(&self) -> Result<(), VoiceError> {
        #[cfg(windows)]
        {
            use wasapi_ffi::*;

            if !self.is_device_available() {
                return Err(VoiceError::DeviceUnavailable(
                    "Nenhum microfone de captura ativo ou acessível no sistema operacional (WASAPI)"
                        .to_string(),
                ));
            }

            if self.is_recording.swap(true, Ordering::SeqCst) {
                return Err(VoiceError::SessionConflict(
                    "Captura de áudio já em andamento".to_string(),
                ));
            }

            let is_recording_flag = self.is_recording.clone();
            let target_device_id = self.device_id.clone();
            let is_diag = self.diagnostics_enabled || crate::voice::diag::is_diagnostics_enabled();

            let (tx_ready, rx_ready) =
                std::sync::mpsc::channel::<Result<NativeAudioFormatInfo, VoiceError>>();

            let handle = std::thread::spawn(move || -> Result<AudioInput, VoiceError> {
                let t_thread_start = Instant::now();
                unsafe {
                    let hr = CoInitializeEx(std::ptr::null_mut(), 0);
                    if hr != 0 && hr != 1 {
                        is_recording_flag.store(false, Ordering::SeqCst);
                        let err = VoiceError::CaptureFailed(format!(
                            "Falha ao inicializar COM na thread de captura (0x{:08X})",
                            hr
                        ));
                        let _ = tx_ready.send(Err(err.clone()));
                        return Err(err);
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
                        is_recording_flag.store(false, Ordering::SeqCst);
                        let err = VoiceError::CaptureFailed(format!(
                            "Falha ao instanciar MMDeviceEnumerator (0x{:08X})",
                            hr
                        ));
                        let _ = tx_ready.send(Err(err.clone()));
                        return Err(err);
                    }

                    let enum_vtbl = *(p_enum as *mut *const IMMDeviceEnumeratorVtbl);
                    let mut p_dev: *mut std::ffi::c_void = std::ptr::null_mut();

                    if let Some(ref dev_id) = target_device_id {
                        let wide_id: Vec<u16> =
                            dev_id.encode_utf16().chain(std::iter::once(0)).collect();
                        let hr = ((*enum_vtbl).GetDevice)(p_enum, wide_id.as_ptr(), &mut p_dev);
                        if hr != 0 || p_dev.is_null() {
                            ((*enum_vtbl).Release)(p_enum);
                            CoUninitialize();
                            is_recording_flag.store(false, Ordering::SeqCst);
                            let err = VoiceError::DeviceUnavailable(format!(
                                "Dispositivo de captura especificado '{}' não encontrado (0x{:08X})",
                                dev_id, hr
                            ));
                            let _ = tx_ready.send(Err(err.clone()));
                            return Err(err);
                        }
                    } else {
                        let hr = ((*enum_vtbl).GetDefaultAudioEndpoint)(
                            p_enum, 1, /* eCapture */
                            0, /* eConsole */
                            &mut p_dev,
                        );
                        if hr != 0 || p_dev.is_null() {
                            ((*enum_vtbl).Release)(p_enum);
                            CoUninitialize();
                            is_recording_flag.store(false, Ordering::SeqCst);
                            let err = VoiceError::DeviceUnavailable(format!(
                                "Nenhum endpoint padrão de captura disponível no Windows (0x{:08X})",
                                hr
                            ));
                            let _ = tx_ready.send(Err(err.clone()));
                            return Err(err);
                        }
                    }

                    let dev_vtbl = *(p_dev as *mut *const IMMDeviceVtbl);
                    let mut dev_state = 0u32;
                    ((*dev_vtbl).GetState)(p_dev, &mut dev_state);
                    if dev_state != 1 {
                        ((*dev_vtbl).Release)(p_dev);
                        ((*enum_vtbl).Release)(p_enum);
                        CoUninitialize();
                        is_recording_flag.store(false, Ordering::SeqCst);
                        let err = VoiceError::DeviceUnavailable(format!(
                            "O microfone selecionado não está no estado ativo (estado do dispositivo: {})",
                            dev_state
                        ));
                        let _ = tx_ready.send(Err(err.clone()));
                        return Err(err);
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
                        is_recording_flag.store(false, Ordering::SeqCst);
                        let err = VoiceError::CaptureFailed(format!(
                            "Falha ao ativar IAudioClient no endpoint (0x{:08X})",
                            hr
                        ));
                        let _ = tx_ready.send(Err(err.clone()));
                        return Err(err);
                    }

                    let client_vtbl = *(p_client as *mut *const IAudioClientVtbl);
                    let mut p_format: *mut WAVEFORMATEX = std::ptr::null_mut();
                    let hr = ((*client_vtbl).GetMixFormat)(p_client, &mut p_format);
                    if hr != 0 || p_format.is_null() {
                        ((*client_vtbl).Release)(p_client);
                        ((*dev_vtbl).Release)(p_dev);
                        ((*enum_vtbl).Release)(p_enum);
                        CoUninitialize();
                        is_recording_flag.store(false, Ordering::SeqCst);
                        let err = VoiceError::CaptureFailed(format!(
                            "Falha ao obter formato nativo do dispositivo (0x{:08X})",
                            hr
                        ));
                        let _ = tx_ready.send(Err(err.clone()));
                        return Err(err);
                    }

                    let fmt_copy = *p_format;
                    let is_extensible = fmt_copy.wFormatTag == 0xFFFE;
                    let mut sub_guid_str = None;
                    let is_float = if is_extensible && fmt_copy.cbSize >= 22 {
                        let ext: WAVEFORMATEXTENSIBLE =
                            std::ptr::read_unaligned(p_format as *const WAVEFORMATEXTENSIBLE);
                        let sub_format: GUID =
                            std::ptr::read_unaligned(std::ptr::addr_of!(ext.sub_format));
                        sub_guid_str = Some(format!(
                            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                            sub_format.data1,
                            sub_format.data2,
                            sub_format.data3,
                            sub_format.data4[0],
                            sub_format.data4[1],
                            sub_format.data4[2],
                            sub_format.data4[3],
                            sub_format.data4[4],
                            sub_format.data4[5],
                            sub_format.data4[6],
                            sub_format.data4[7],
                        ));
                        sub_format == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT
                    } else {
                        fmt_copy.wFormatTag == 3
                    };

                    let format_tag_name = if is_extensible {
                        if is_float {
                            "WAVE_FORMAT_EXTENSIBLE (IEEE Float)"
                        } else {
                            "WAVE_FORMAT_EXTENSIBLE (PCM)"
                        }
                    } else if fmt_copy.wFormatTag == 3 {
                        "WAVE_FORMAT_IEEE_FLOAT"
                    } else if fmt_copy.wFormatTag == 1 {
                        "WAVE_FORMAT_PCM"
                    } else {
                        "WAVE_FORMAT_UNKNOWN"
                    }
                    .to_string();

                    let native_info = NativeAudioFormatInfo {
                        format_tag: fmt_copy.wFormatTag,
                        format_tag_hex: format!("0x{:04X}", fmt_copy.wFormatTag),
                        format_tag_name,
                        is_extensible,
                        is_float,
                        channels: fmt_copy.nChannels,
                        sample_rate: fmt_copy.nSamplesPerSec,
                        bits_per_sample: fmt_copy.wBitsPerSample,
                        block_align: fmt_copy.nBlockAlign,
                        avg_bytes_per_sec: fmt_copy.nAvgBytesPerSec,
                        sub_format_guid: sub_guid_str,
                    };

                    // AUDCLNT_SHAREMODE_SHARED = 0. Buffer = 1 segundo (10_000_000 * 100ns)
                    let hr = ((*client_vtbl).Initialize)(
                        p_client,
                        0,
                        0,
                        10_000_000,
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
                        is_recording_flag.store(false, Ordering::SeqCst);
                        let err = VoiceError::CaptureFailed(format!(
                            "Falha ao inicializar IAudioClient em modo compartilhado (0x{:08X})",
                            hr
                        ));
                        let _ = tx_ready.send(Err(err.clone()));
                        return Err(err);
                    }

                    let mut p_capture: *mut std::ffi::c_void = std::ptr::null_mut();
                    let hr = ((*client_vtbl).GetService)(
                        p_client,
                        &IID_IAUDIO_CAPTURE_CLIENT,
                        &mut p_capture,
                    );
                    if hr != 0 || p_capture.is_null() {
                        CoTaskMemFree(p_format as *mut std::ffi::c_void);
                        ((*client_vtbl).Release)(p_client);
                        ((*dev_vtbl).Release)(p_dev);
                        ((*enum_vtbl).Release)(p_enum);
                        CoUninitialize();
                        is_recording_flag.store(false, Ordering::SeqCst);
                        let err = VoiceError::CaptureFailed(format!(
                            "Falha ao obter IAudioCaptureClient (0x{:08X})",
                            hr
                        ));
                        let _ = tx_ready.send(Err(err.clone()));
                        return Err(err);
                    }

                    let cap_vtbl = *(p_capture as *mut *const IAudioCaptureClientVtbl);
                    let hr = ((*client_vtbl).Start)(p_client);
                    if hr != 0 {
                        ((*cap_vtbl).Release)(p_capture);
                        CoTaskMemFree(p_format as *mut std::ffi::c_void);
                        ((*client_vtbl).Release)(p_client);
                        ((*dev_vtbl).Release)(p_dev);
                        ((*enum_vtbl).Release)(p_enum);
                        CoUninitialize();
                        is_recording_flag.store(false, Ordering::SeqCst);
                        let err = VoiceError::CaptureFailed(format!(
                            "Falha ao iniciar stream de captura de áudio (0x{:08X})",
                            hr
                        ));
                        let _ = tx_ready.send(Err(err.clone()));
                        return Err(err);
                    }

                    let t_stream_start = Instant::now();
                    // Notifica a thread principal que o hardware está efetivamente capturando frames
                    let _ = tx_ready.send(Ok(native_info.clone()));

                    let mut raw_bytes: Vec<u8> =
                        Vec::with_capacity(fmt_copy.nAvgBytesPerSec as usize * 5);
                    let block_align = fmt_copy.nBlockAlign as usize;
                    let mut total_packets = 0usize;
                    let mut total_raw_frames = 0usize;
                    let mut silent_buffer_events = 0usize;
                    let mut discontinuity_glitches = 0usize;

                    while is_recording_flag.load(Ordering::SeqCst) {
                        let mut packet_size = 0u32;
                        let _ = ((*cap_vtbl).GetNextPacketSize)(p_capture, &mut packet_size);
                        while packet_size > 0 {
                            let mut p_data: *mut u8 = std::ptr::null_mut();
                            let mut num_frames = 0u32;
                            let mut flags = 0u32;
                            let mut dev_pos = 0u64;
                            let mut qpc_pos = 0u64;
                            let hr = ((*cap_vtbl).GetBuffer)(
                                p_capture,
                                &mut p_data,
                                &mut num_frames,
                                &mut flags,
                                &mut dev_pos,
                                &mut qpc_pos,
                            );
                            if hr == 0 && num_frames > 0 && !p_data.is_null() {
                                total_packets += 1;
                                total_raw_frames += num_frames as usize;
                                if flags & 1 != 0 {
                                    discontinuity_glitches += 1;
                                }
                                let byte_count = (num_frames as usize) * block_align;
                                if flags & 2 != 0 {
                                    // AUDCLNT_BUFFERFLAGS_SILENT
                                    silent_buffer_events += 1;
                                    raw_bytes.resize(raw_bytes.len() + byte_count, 0);
                                } else {
                                    let slice = std::slice::from_raw_parts(p_data, byte_count);
                                    raw_bytes.extend_from_slice(slice);
                                }
                                let _ = ((*cap_vtbl).ReleaseBuffer)(p_capture, num_frames);
                            }
                            packet_size = 0;
                            let _ = ((*cap_vtbl).GetNextPacketSize)(p_capture, &mut packet_size);
                        }
                        std::thread::sleep(Duration::from_millis(15));
                    }

                    // Encerramento limpo e drenagem de buffers finais
                    let _ = ((*client_vtbl).Stop)(p_client);
                    let t_stream_stop = Instant::now();

                    let mut packet_size = 0u32;
                    let _ = ((*cap_vtbl).GetNextPacketSize)(p_capture, &mut packet_size);
                    while packet_size > 0 {
                        let mut p_data: *mut u8 = std::ptr::null_mut();
                        let mut num_frames = 0u32;
                        let mut flags = 0u32;
                        let mut dev_pos = 0u64;
                        let mut qpc_pos = 0u64;
                        let hr = ((*cap_vtbl).GetBuffer)(
                            p_capture,
                            &mut p_data,
                            &mut num_frames,
                            &mut flags,
                            &mut dev_pos,
                            &mut qpc_pos,
                        );
                        if hr == 0 && num_frames > 0 && !p_data.is_null() {
                            total_packets += 1;
                            total_raw_frames += num_frames as usize;
                            if flags & 1 != 0 {
                                discontinuity_glitches += 1;
                            }
                            let byte_count = (num_frames as usize) * block_align;
                            if flags & 2 != 0 {
                                silent_buffer_events += 1;
                                raw_bytes.resize(raw_bytes.len() + byte_count, 0);
                            } else {
                                let slice = std::slice::from_raw_parts(p_data, byte_count);
                                raw_bytes.extend_from_slice(slice);
                            }
                            let _ = ((*cap_vtbl).ReleaseBuffer)(p_capture, num_frames);
                        }
                        packet_size = 0;
                        let _ = ((*cap_vtbl).GetNextPacketSize)(p_capture, &mut packet_size);
                    }

                    ((*cap_vtbl).Release)(p_capture);
                    CoTaskMemFree(p_format as *mut std::ffi::c_void);
                    ((*client_vtbl).Release)(p_client);
                    ((*dev_vtbl).Release)(p_dev);
                    ((*enum_vtbl).Release)(p_enum);
                    CoUninitialize();

                    // Conversão de formato para mono i16 na taxa nativa
                    let channels = fmt_copy.nChannels as usize;
                    let native_rate = fmt_copy.nSamplesPerSec;
                    let bits = fmt_copy.wBitsPerSample;

                    let mut mono_samples: Vec<i16> = Vec::new();

                    if is_float && bits == 32 {
                        let frame_size = channels * 4;
                        for frame in raw_bytes.chunks_exact(frame_size) {
                            let mut sum = 0.0f32;
                            for c in 0..channels {
                                let sample_bytes = [
                                    frame[c * 4],
                                    frame[c * 4 + 1],
                                    frame[c * 4 + 2],
                                    frame[c * 4 + 3],
                                ];
                                sum += f32::from_le_bytes(sample_bytes);
                            }
                            let avg = sum / (channels as f32);
                            mono_samples.push((avg.clamp(-1.0, 1.0) * 32767.0) as i16);
                        }
                    } else if bits == 16 {
                        let frame_size = channels * 2;
                        for frame in raw_bytes.chunks_exact(frame_size) {
                            let mut sum = 0i32;
                            for c in 0..channels {
                                let sample_bytes = [frame[c * 2], frame[c * 2 + 1]];
                                sum += i16::from_le_bytes(sample_bytes) as i32;
                            }
                            let avg = sum / (channels as i32);
                            mono_samples.push(avg.clamp(-32768, 32767) as i16);
                        }
                    } else if bits == 24 {
                        let frame_size = channels * 3;
                        for frame in raw_bytes.chunks_exact(frame_size) {
                            let mut sum = 0i32;
                            for c in 0..channels {
                                let b = &frame[c * 3..c * 3 + 3];
                                let sign_ext = if b[2] & 0x80 != 0 { 0xFF } else { 0x00 };
                                let val = i32::from_le_bytes([b[0], b[1], b[2], sign_ext]) >> 8;
                                sum += val;
                            }
                            let avg = sum / (channels as i32);
                            mono_samples.push(avg.clamp(-32768, 32767) as i16);
                        }
                    } else if bits == 32 {
                        let frame_size = channels * 4;
                        for frame in raw_bytes.chunks_exact(frame_size) {
                            let mut sum = 0i64;
                            for c in 0..channels {
                                let sample_bytes = [
                                    frame[c * 4],
                                    frame[c * 4 + 1],
                                    frame[c * 4 + 2],
                                    frame[c * 4 + 3],
                                ];
                                sum += (i32::from_le_bytes(sample_bytes) >> 16) as i64;
                            }
                            let avg = sum / (channels as i64);
                            mono_samples.push(avg.clamp(-32768, 32767) as i16);
                        }
                    } else {
                        return Err(VoiceError::InvalidAudioFormat(format!(
                            "Profundidade de bits não suportada para conversão de microfone: {} bits",
                            bits
                        )));
                    }

                    // Reamostragem para a taxa canônica de 16 kHz mono da Yuki
                    let canonical_samples =
                        Self::resample_linear(&mono_samples, native_rate, 16000);
                    let canonical_format = AudioFormat::default();
                    let wav_payload =
                        AudioCodec::encode_wav(&canonical_samples, &canonical_format)?;
                    let samples_duration_ms = (canonical_samples.len() as u64 * 1000) / 16000;

                    let init_duration_ms =
                        t_stream_start.duration_since(t_thread_start).as_millis() as u64;
                    let effective_capture_ms =
                        t_stream_stop.duration_since(t_stream_start).as_millis() as u64;

                    let buffer_metrics = BufferMetrics {
                        total_packets,
                        total_raw_frames,
                        total_raw_bytes: raw_bytes.len(),
                        silent_buffer_events,
                        discontinuity_glitches,
                    };

                    let source_label = format!("wasapi-mic-{}", native_rate);
                    let mut audio_input = AudioInput::new(
                        wav_payload,
                        canonical_format,
                        samples_duration_ms,
                        source_label,
                    );

                    if is_diag {
                        let session_id = generate_unique_id("diag");
                        let timings = CaptureTimingMetrics {
                            requested_duration_ms: effective_capture_ms,
                            init_duration_ms,
                            effective_capture_ms,
                            samples_duration_ms,
                        };
                        let diag_report = crate::voice::diag::save_diagnostic_session(
                            &session_id,
                            target_device_id.clone(),
                            None,
                            &native_info,
                            timings,
                            buffer_metrics,
                            &raw_bytes,
                            &canonical_samples,
                        )?;
                        audio_input = audio_input.with_diagnostics(diag_report);
                    }

                    Ok(audio_input)
                }
            });

            match rx_ready.recv_timeout(Duration::from_secs(5)) {
                Ok(Ok(_info)) => {
                    *self.worker_handle.lock().unwrap() = Some(handle);
                    Ok(())
                }
                Ok(Err(e)) => {
                    self.is_recording.store(false, Ordering::SeqCst);
                    let _ = handle.join();
                    Err(e)
                }
                Err(_) => {
                    self.is_recording.store(false, Ordering::SeqCst);
                    Err(VoiceError::CaptureFailed(
                        "Timeout ao aguardar inicialização do dispositivo de áudio WASAPI (5s)"
                            .to_string(),
                    ))
                }
            }
        }
        #[cfg(not(windows))]
        {
            Err(VoiceError::CaptureFailed(
                "WasapiAudioCaptureBackend é suportado apenas no Windows".to_string(),
            ))
        }
    }

    fn stop_capture(&self) -> Result<AudioInput, VoiceError> {
        if !self.is_recording.swap(false, Ordering::SeqCst) {
            return Err(VoiceError::SessionConflict(
                "Nenhuma gravação de áudio ativa para encerrar".to_string(),
            ));
        }

        let handle = self.worker_handle.lock().unwrap().take().ok_or_else(|| {
            VoiceError::CaptureFailed("Worker thread WASAPI não encontrada".to_string())
        })?;

        match handle.join() {
            Ok(result) => result,
            Err(_) => Err(VoiceError::CaptureFailed(
                "Thread de captura WASAPI encerrou inesperadamente (panic)".to_string(),
            )),
        }
    }

    fn cancel_capture(&self) -> Result<(), VoiceError> {
        self.is_recording.store(false, Ordering::SeqCst);
        if let Some(handle) = self.worker_handle.lock().unwrap().take() {
            let _ = handle.join();
        }
        Ok(())
    }

    fn is_device_available(&self) -> bool {
        #[cfg(windows)]
        {
            use crate::voice::devices::{list_audio_devices, AudioDeviceFlow};
            if let Ok(devs) = list_audio_devices(true) {
                if let Some(ref target_id) = self.device_id {
                    devs.iter().any(|d| {
                        d.flow == AudioDeviceFlow::Input && d.id.eq_ignore_ascii_case(target_id)
                    })
                } else {
                    devs.iter().any(|d| d.flow == AudioDeviceFlow::Input)
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
        "WindowsAudioCaptureBackend (WASAPI Core Audio)"
    }
}

// ---------------------------------------------------------------------------
// Backend de Compatibilidade e Transição (WindowsAudioCaptureBackend)
// ---------------------------------------------------------------------------

/// Backend padrão de captura para Windows.
///
/// Utiliza o backend moderno WASAPI Core Audio por padrão, suportando seleção
/// determinística de microfones (incluindo Bluetooth Hands-Free e USB) e garantindo
/// operação confiável sem falhas de drivers legados MCI.
#[derive(Debug)]
pub struct WindowsAudioCaptureBackend {
    inner: WasapiAudioCaptureBackend,
}

impl Default for WindowsAudioCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsAudioCaptureBackend {
    pub fn new() -> Self {
        Self {
            inner: WasapiAudioCaptureBackend::new(None),
        }
    }

    pub fn with_device(device_id: Option<String>) -> Self {
        Self {
            inner: WasapiAudioCaptureBackend::new(device_id),
        }
    }

    pub fn with_diagnostics(device_id: Option<String>, diagnostics: bool) -> Self {
        Self {
            inner: WasapiAudioCaptureBackend::with_diagnostics(device_id, diagnostics),
        }
    }
}

impl AudioCaptureBackend for WindowsAudioCaptureBackend {
    fn start_capture(&self) -> Result<(), VoiceError> {
        self.inner.start_capture()
    }

    fn stop_capture(&self) -> Result<AudioInput, VoiceError> {
        self.inner.stop_capture()
    }

    fn cancel_capture(&self) -> Result<(), VoiceError> {
        self.inner.cancel_capture()
    }

    fn is_device_available(&self) -> bool {
        self.inner.is_device_available()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

// ---------------------------------------------------------------------------
// Backend Legado MCI Corrigido (MciAudioCaptureBackend)
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[allow(clippy::upper_case_acronyms)]
mod winmm_mci {
    use std::os::raw::{c_char, c_void};
    pub type UINT = u32;
    pub type LPCSTR = *const c_char;
    pub type LPSTR = *mut c_char;
    pub type HWND = *mut c_void;

    #[link(name = "winmm")]
    extern "system" {
        pub fn mciSendStringA(
            lpstrCommand: LPCSTR,
            lpstrReturnString: LPSTR,
            uReturnLength: UINT,
            hwndCallback: HWND,
        ) -> u32;
        pub fn mciGetErrorStringA(fdwError: u32, lpszErrorText: LPSTR, cchErrorText: UINT) -> i32;
        pub fn waveInGetNumDevs() -> UINT;
    }
}

/// Backend legado MCI corrigido, com alinhamento de bloco estrito (`set alignment 2`)
/// e tradução formal de códigos de erro do Windows via `mciGetErrorStringA`.
#[derive(Debug)]
pub struct MciAudioCaptureBackend {
    alias: Arc<Mutex<Option<String>>>,
    temp_path: Arc<Mutex<Option<std::path::PathBuf>>>,
    is_recording: Arc<AtomicBool>,
}

impl Default for MciAudioCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MciAudioCaptureBackend {
    pub fn new() -> Self {
        Self {
            alias: Arc::new(Mutex::new(None)),
            temp_path: Arc::new(Mutex::new(None)),
            is_recording: Arc::new(AtomicBool::new(false)),
        }
    }

    #[cfg(windows)]
    fn send_mci(cmd: &str) -> Result<(), VoiceError> {
        use std::ffi::{CStr, CString};
        let c_cmd = CString::new(cmd).map_err(|e| {
            VoiceError::CaptureFailed(format!("String de comando MCI inválida: {}", e))
        })?;
        let code = unsafe {
            winmm_mci::mciSendStringA(
                c_cmd.as_ptr(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            )
        };
        if code != 0 {
            let mut err_buf = [0i8; 512];
            let msg = unsafe {
                if winmm_mci::mciGetErrorStringA(code, err_buf.as_mut_ptr(), 512) != 0 {
                    CStr::from_ptr(err_buf.as_ptr())
                        .to_string_lossy()
                        .into_owned()
                } else {
                    format!("Erro desconhecido MCI {}", code)
                }
            };
            Err(VoiceError::CaptureFailed(format!(
                "Falha na API MCI do Windows (código {} - '{}'): comando '{}'",
                code, msg, cmd
            )))
        } else {
            Ok(())
        }
    }
}

impl AudioCaptureBackend for MciAudioCaptureBackend {
    fn start_capture(&self) -> Result<(), VoiceError> {
        #[cfg(windows)]
        {
            if !self.is_device_available() {
                return Err(VoiceError::DeviceUnavailable(
                    "Nenhum microfone detectado ou acessível no sistema operacional (waveIn)"
                        .to_string(),
                ));
            }

            if self.is_recording.swap(true, Ordering::SeqCst) {
                return Err(VoiceError::SessionConflict(
                    "Captura de áudio já em andamento".to_string(),
                ));
            }

            let session_id = generate_unique_id("rec");
            let alias = format!("yuki_{}", session_id);
            let temp_file = std::env::temp_dir().join(format!("yuki_{}.wav", session_id));

            let _ = Self::send_mci(&format!("close {}", alias));

            let setup_result = (|| {
                Self::send_mci(&format!("open new type waveaudio alias {}", alias))?;
                Self::send_mci(&format!("set {} time format ms", alias))?;
                Self::send_mci(&format!("set {} format tag pcm", alias))?;
                Self::send_mci(&format!("set {} channels 1", alias))?;
                Self::send_mci(&format!("set {} samplespersec 16000", alias))?;
                Self::send_mci(&format!("set {} bitspersample 16", alias))?;
                // Invariante de alinhamento: 1 canal * 16 bits / 8 = 2 bytes
                Self::send_mci(&format!("set {} alignment 2", alias))?;
                Self::send_mci(&format!("record {}", alias))?;
                Ok::<(), VoiceError>(())
            })();

            if let Err(e) = setup_result {
                let _ = Self::send_mci(&format!("close {}", alias));
                self.is_recording.store(false, Ordering::SeqCst);
                return Err(e);
            }

            *self.alias.lock().unwrap() = Some(alias);
            *self.temp_path.lock().unwrap() = Some(temp_file);

            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(VoiceError::CaptureFailed(
                "MciAudioCaptureBackend é suportado apenas no Windows".to_string(),
            ))
        }
    }

    fn stop_capture(&self) -> Result<AudioInput, VoiceError> {
        #[cfg(windows)]
        {
            if !self.is_recording.swap(false, Ordering::SeqCst) {
                return Err(VoiceError::SessionConflict(
                    "Nenhuma gravação de áudio em andamento para encerrar".to_string(),
                ));
            }

            let alias =
                self.alias.lock().unwrap().take().ok_or_else(|| {
                    VoiceError::CaptureFailed("Alias MCI não encontrado".to_string())
                })?;
            let temp_path = self.temp_path.lock().unwrap().take().ok_or_else(|| {
                VoiceError::CaptureFailed("Caminho temporário de gravação ausente".to_string())
            })?;

            let stop_result = (|| {
                Self::send_mci(&format!("stop {}", alias))?;
                let path_str = temp_path.to_string_lossy().replace('\\', "/");
                Self::send_mci(&format!("save {} \"{}\"", alias, path_str))?;
                Self::send_mci(&format!("close {}", alias))?;
                Ok::<(), VoiceError>(())
            })();

            if let Err(e) = stop_result {
                let _ = Self::send_mci(&format!("close {}", alias));
                let _ = std::fs::remove_file(&temp_path);
                return Err(e);
            }

            let wav_bytes = std::fs::read(&temp_path).map_err(|e| {
                VoiceError::Io(format!("Falha ao ler áudio gravado do microfone: {}", e))
            })?;
            let _ = std::fs::remove_file(&temp_path);

            let (format, samples) = AudioCodec::decode_wav(&wav_bytes)?;
            let duration_ms = if format.sample_rate > 0 && format.channels > 0 {
                ((samples.len() as u64) * 1000)
                    / (format.sample_rate as u64 * format.channels as u64)
            } else {
                0
            };

            Ok(AudioInput::new(
                wav_bytes,
                format,
                duration_ms,
                "microphone-windows-mci",
            ))
        }
        #[cfg(not(windows))]
        {
            Err(VoiceError::CaptureFailed(
                "MciAudioCaptureBackend é suportado apenas no Windows".to_string(),
            ))
        }
    }

    fn cancel_capture(&self) -> Result<(), VoiceError> {
        #[cfg(windows)]
        {
            if self.is_recording.swap(false, Ordering::SeqCst) {
                if let Some(alias) = self.alias.lock().unwrap().take() {
                    let _ = Self::send_mci(&format!("stop {}", alias));
                    let _ = Self::send_mci(&format!("close {}", alias));
                }
                if let Some(temp_path) = self.temp_path.lock().unwrap().take() {
                    let _ = std::fs::remove_file(&temp_path);
                }
            }
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
            let devs = unsafe { winmm_mci::waveInGetNumDevs() };
            devs > 0
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    fn name(&self) -> &str {
        "MciAudioCaptureBackend (WinMM MCI)"
    }
}

// ---------------------------------------------------------------------------
// Backend Mock Observável para Testes e CI Headless
// ---------------------------------------------------------------------------

/// Backend mock determinístico para testes e validação sem microfone físico.
#[derive(Debug)]
pub struct MockCaptureBackend {
    device_available: Arc<AtomicBool>,
    is_recording: Arc<AtomicBool>,
    configured_input: Arc<Mutex<Option<AudioInput>>>,
    start_count: Arc<AtomicUsize>,
    stop_count: Arc<AtomicUsize>,
    cancel_count: Arc<AtomicUsize>,
}

impl Default for MockCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCaptureBackend {
    pub fn new() -> Self {
        Self {
            device_available: Arc::new(AtomicBool::new(true)),
            is_recording: Arc::new(AtomicBool::new(false)),
            configured_input: Arc::new(Mutex::new(None)),
            start_count: Arc::new(AtomicUsize::new(0)),
            stop_count: Arc::new(AtomicUsize::new(0)),
            cancel_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn set_device_available(&self, available: bool) {
        self.device_available.store(available, Ordering::SeqCst);
    }

    pub fn set_mock_input(&self, input: AudioInput) {
        *self.configured_input.lock().unwrap() = Some(input);
    }

    pub fn start_count(&self) -> usize {
        self.start_count.load(Ordering::SeqCst)
    }

    pub fn stop_count(&self) -> usize {
        self.stop_count.load(Ordering::SeqCst)
    }

    pub fn cancel_count(&self) -> usize {
        self.cancel_count.load(Ordering::SeqCst)
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }
}

impl AudioCaptureBackend for MockCaptureBackend {
    fn start_capture(&self) -> Result<(), VoiceError> {
        if !self.is_device_available() {
            return Err(VoiceError::DeviceUnavailable(
                "Microfone mock configurado como indisponível".to_string(),
            ));
        }
        if self.is_recording.swap(true, Ordering::SeqCst) {
            return Err(VoiceError::SessionConflict(
                "Captura mock já em andamento".to_string(),
            ));
        }
        self.start_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn stop_capture(&self) -> Result<AudioInput, VoiceError> {
        if !self.is_recording.swap(false, Ordering::SeqCst) {
            return Err(VoiceError::SessionConflict(
                "Nenhuma captura mock ativa para encerrar".to_string(),
            ));
        }
        self.stop_count.fetch_add(1, Ordering::SeqCst);

        if let Some(input) = self.configured_input.lock().unwrap().clone() {
            Ok(input)
        } else {
            // Gera 1 segundo de silêncio codificado em WAV canônico
            let samples = vec![0i16; 16000];
            let format = AudioFormat::default();
            let wav_bytes = AudioCodec::encode_wav(&samples, &format)?;
            Ok(AudioInput::new(wav_bytes, format, 1000, "mock-microphone"))
        }
    }

    fn cancel_capture(&self) -> Result<(), VoiceError> {
        self.is_recording.store(false, Ordering::SeqCst);
        self.cancel_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn is_device_available(&self) -> bool {
        self.device_available.load(Ordering::SeqCst)
    }

    fn name(&self) -> &str {
        "MockCaptureBackend"
    }
}

// ---------------------------------------------------------------------------
// Controlador de Push-to-Talk Seguro
// ---------------------------------------------------------------------------

/// Controlador de ciclo de vida para operação Push-to-Talk (PTT) com garantias de segurança:
/// - Limite estrito de tempo de fala contínua (padrão 30s) para evitar estouro de memória.
/// - Limite máximo de tamanho de payload em bytes.
/// - Validação de integridade do buffer capturado antes da entrega ao subsistema STT.
#[derive(Debug)]
pub struct PushToTalkController {
    backend: Arc<dyn AudioCaptureBackend>,
    max_duration: Duration,
    max_bytes: usize,
    started_at: Arc<Mutex<Option<Instant>>>,
    is_active: Arc<AtomicBool>,
}

impl PushToTalkController {
    pub const DEFAULT_MAX_DURATION_SECS: u64 = 30;
    /// 30 segundos * 16000 amostras/s * 2 bytes/amostra = 960.000 bytes PCM + header WAV (~1MB)
    pub const DEFAULT_MAX_BYTES: usize = 1_000_000;

    pub fn new(backend: Arc<dyn AudioCaptureBackend>) -> Self {
        Self {
            backend,
            max_duration: Duration::from_secs(Self::DEFAULT_MAX_DURATION_SECS),
            max_bytes: Self::DEFAULT_MAX_BYTES,
            started_at: Arc::new(Mutex::new(None)),
            is_active: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_limits(mut self, max_duration: Duration, max_bytes: usize) -> Self {
        self.max_duration = max_duration;
        self.max_bytes = max_bytes;
        self
    }

    pub fn backend(&self) -> &Arc<dyn AudioCaptureBackend> {
        &self.backend
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.started_at.lock().unwrap().map(|t| t.elapsed())
    }

    /// Pressionar o botão / iniciar captura Push-to-Talk.
    pub fn press(&self) -> Result<(), VoiceError> {
        if !self.backend.is_device_available() {
            return Err(VoiceError::DeviceUnavailable(
                "Microfone indisponível ou desconectado para Push-to-Talk".to_string(),
            ));
        }

        if self.is_active.swap(true, Ordering::SeqCst) {
            return Err(VoiceError::SessionConflict(
                "Sessão Push-to-Talk já está ativa".to_string(),
            ));
        }

        if let Err(e) = self.backend.start_capture() {
            self.is_active.store(false, Ordering::SeqCst);
            return Err(e);
        }

        *self.started_at.lock().unwrap() = Some(Instant::now());
        Ok(())
    }

    /// Soltar o botão / finalizar captura Push-to-Talk e validar integridade e limites.
    pub fn release(&self) -> Result<AudioInput, VoiceError> {
        if !self.is_active.swap(false, Ordering::SeqCst) {
            return Err(VoiceError::SessionConflict(
                "Nenhuma sessão Push-to-Talk ativa para finalizar".to_string(),
            ));
        }

        let started = self.started_at.lock().unwrap().take();
        let elapsed = started.map(|t| t.elapsed()).unwrap_or_default();

        let mut audio_input = self.backend.stop_capture()?;

        if let Some(ref mut diag) = audio_input.diagnostics {
            diag.timings.requested_duration_ms = elapsed.as_millis() as u64;
            if let Some(ref path_str) = diag.report_path {
                if let Ok(json_bytes) = serde_json::to_vec_pretty(&diag) {
                    let _ = std::fs::write(path_str, json_bytes);
                }
            }
        }

        // Validação de limite estrito de tamanho
        if audio_input.payload.len() > self.max_bytes {
            return Err(VoiceError::AudioTooLarge {
                bytes: audio_input.payload.len(),
                max_bytes: self.max_bytes,
            });
        }

        // Validação de duração máxima
        if elapsed > self.max_duration {
            return Err(VoiceError::AudioTooLong {
                duration_ms: elapsed.as_millis() as u64,
                max_ms: self.max_duration.as_millis() as u64,
            });
        }

        // Validação de payload não-vazio
        if audio_input.payload.is_empty() {
            return Err(VoiceError::InvalidAudioFormat(
                "Nenhum áudio foi gravado durante o período Push-to-Talk".to_string(),
            ));
        }

        Ok(audio_input)
    }

    /// Cancela a captura imediatamente sem retornar dados.
    pub fn cancel(&self) -> Result<(), VoiceError> {
        self.is_active.store(false, Ordering::SeqCst);
        *self.started_at.lock().unwrap() = None;
        self.backend.cancel_capture()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_capture_lifecycle() {
        let backend = MockCaptureBackend::new();
        assert!(backend.is_device_available());
        assert!(!backend.is_recording());

        assert!(backend.start_capture().is_ok());
        assert!(backend.is_recording());
        assert_eq!(backend.start_count(), 1);

        assert!(matches!(
            backend.start_capture(),
            Err(VoiceError::SessionConflict(_))
        ));

        let audio = backend.stop_capture().expect("deve parar com sucesso");
        assert_eq!(backend.stop_count(), 1);
        assert!(!backend.is_recording());
        assert!(!audio.payload.is_empty());
        assert_eq!(audio.format.sample_rate, 16000);
    }

    #[test]
    fn test_ptt_controller_lifecycle_and_guards() {
        let mock_backend = Arc::new(MockCaptureBackend::new());
        let ptt = PushToTalkController::new(mock_backend.clone());

        assert!(!ptt.is_active());
        assert!(ptt.press().is_ok());
        assert!(ptt.is_active());

        assert!(matches!(ptt.press(), Err(VoiceError::SessionConflict(_))));

        let audio = ptt.release().expect("deve liberar com sucesso");
        assert!(!ptt.is_active());
        assert_eq!(audio.duration_ms, 1000);

        assert!(matches!(ptt.release(), Err(VoiceError::SessionConflict(_))));
    }

    #[test]
    fn test_ptt_payload_limit_enforcement() {
        let mock_backend = Arc::new(MockCaptureBackend::new());
        let ptt = PushToTalkController::new(mock_backend.clone())
            .with_limits(Duration::from_secs(30), 100);

        let large_input =
            AudioInput::new(vec![0u8; 500], AudioFormat::default(), 100, "mock-large");
        mock_backend.set_mock_input(large_input);

        ptt.press().unwrap();
        let res = ptt.release();
        assert!(matches!(res, Err(VoiceError::AudioTooLarge { .. })));
    }

    #[test]
    fn test_ptt_device_unavailable() {
        let mock_backend = Arc::new(MockCaptureBackend::new());
        mock_backend.set_device_available(false);
        let ptt = PushToTalkController::new(mock_backend);

        assert!(matches!(ptt.press(), Err(VoiceError::DeviceUnavailable(_))));
    }

    #[test]
    fn test_resample_linear_identity_and_conversion() {
        let input: Vec<i16> = (0..48000).map(|i| (i % 1000) as i16).collect();
        let resampled = WasapiAudioCaptureBackend::resample_linear(&input, 48000, 16000);
        assert_eq!(resampled.len(), 16000);

        let identity = WasapiAudioCaptureBackend::resample_linear(&input, 16000, 16000);
        assert_eq!(identity.len(), input.len());
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_capture_device_detection() {
        let backend = WindowsAudioCaptureBackend::new();
        let available = backend.is_device_available();
        println!("Windows audio capture device available: {}", available);
    }
}
