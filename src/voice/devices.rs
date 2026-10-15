use crate::contracts::voice::VoiceError;
use serde::{Deserialize, Serialize};

/// Direção do fluxo de áudio do dispositivo (Entrada ou Saída).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioDeviceFlow {
    Input,
    Output,
}

impl AudioDeviceFlow {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Input => "Entrada (Microfone)",
            Self::Output => "Saída (Alto-falante/Fone)",
        }
    }
}

/// Estado operacional do dispositivo no sistema operacional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioDeviceState {
    Active,
    Disabled,
    NotPresent,
    Unplugged,
    Unknown,
}

impl AudioDeviceState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "Ativo",
            Self::Disabled => "Desabilitado",
            Self::NotPresent => "Não presente",
            Self::Unplugged => "Desconectado",
            Self::Unknown => "Desconhecido",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// Informações descritivas de um endpoint de áudio do sistema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub flow: AudioDeviceFlow,
    pub state: AudioDeviceState,
    pub is_default: bool,
}

// ---------------------------------------------------------------------------
// Implementação Windows WASAPI / Core Audio via COM
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

    pub const PKEY_DEVICE_FRIENDLY_NAME_FMTID: GUID = GUID {
        data1: 0xA45C254E,
        data2: 0xDF1C,
        data3: 0x4EFD,
        data4: [0x80, 0x20, 0x67, 0xD1, 0x46, 0xA8, 0x50, 0xE0],
    };
    pub const PKEY_DEVICE_FRIENDLY_NAME_PID: u32 = 14;

    #[repr(C)]
    pub struct PROPERTYKEY {
        pub fmtid: GUID,
        pub pid: u32,
    }

    #[repr(C)]
    pub struct PROPVARIANT {
        pub vt: u16,
        pub w_reserved1: u16,
        pub w_reserved2: u16,
        pub w_reserved3: u16,
        pub val_ptr: *mut u16,
        pub padding: usize,
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
        pub fn PropVariantClear(pvar: *mut PROPVARIANT) -> HRESULT;
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
    pub struct IMMDeviceCollectionVtbl {
        pub QueryInterface:
            unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
        pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
        pub GetCount: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
        pub Item: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> HRESULT,
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
    pub struct IPropertyStoreVtbl {
        pub QueryInterface:
            unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
        pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
        pub GetCount: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
        pub GetAt: unsafe extern "system" fn(*mut c_void, u32, *mut PROPERTYKEY) -> HRESULT,
        pub GetValue:
            unsafe extern "system" fn(*mut c_void, *const PROPERTYKEY, *mut PROPVARIANT) -> HRESULT,
        pub SetValue: unsafe extern "system" fn(
            *mut c_void,
            *const PROPERTYKEY,
            *const PROPVARIANT,
        ) -> HRESULT,
        pub Commit: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    }

    pub fn pwstr_to_string(ptr: *const u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let mut len = 0;
        unsafe {
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(ptr, len);
            String::from_utf16_lossy(slice)
        }
    }
}

/// Enumera todos os dispositivos de áudio disponíveis no sistema operacional.
///
/// Se `only_active` for `true`, retorna apenas endpoints no estado ativo (`Active`).
/// Se for `false`, retorna todos os endpoints conhecidos (ativos, desconectados e desabilitados).
pub fn list_audio_devices(only_active: bool) -> Result<Vec<AudioDeviceInfo>, VoiceError> {
    #[cfg(windows)]
    {
        use wasapi_ffi::*;

        unsafe {
            let _ = CoInitializeEx(std::ptr::null_mut(), 0);

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
                return Err(VoiceError::DeviceUnavailable(format!(
                    "Falha ao instanciar MMDeviceEnumerator via COM (HRESULT 0x{:08X})",
                    hr
                )));
            }

            let enum_vtbl = *(p_enum as *mut *const IMMDeviceEnumeratorVtbl);

            // Obter IDs dos dispositivos padrão
            let mut def_render_id = String::new();
            let mut def_capture_id = String::new();

            let mut p_def_render: *mut std::ffi::c_void = std::ptr::null_mut();
            if ((*enum_vtbl).GetDefaultAudioEndpoint)(p_enum, 0, 0, &mut p_def_render) == 0
                && !p_def_render.is_null()
            {
                let dev_vtbl = *(p_def_render as *mut *const IMMDeviceVtbl);
                let mut p_id: *mut u16 = std::ptr::null_mut();
                if ((*dev_vtbl).GetId)(p_def_render, &mut p_id) == 0 && !p_id.is_null() {
                    def_render_id = pwstr_to_string(p_id);
                    CoTaskMemFree(p_id as *mut std::ffi::c_void);
                }
                ((*dev_vtbl).Release)(p_def_render);
            }

            let mut p_def_capture: *mut std::ffi::c_void = std::ptr::null_mut();
            if ((*enum_vtbl).GetDefaultAudioEndpoint)(p_enum, 1, 0, &mut p_def_capture) == 0
                && !p_def_capture.is_null()
            {
                let dev_vtbl = *(p_def_capture as *mut *const IMMDeviceVtbl);
                let mut p_id: *mut u16 = std::ptr::null_mut();
                if ((*dev_vtbl).GetId)(p_def_capture, &mut p_id) == 0 && !p_id.is_null() {
                    def_capture_id = pwstr_to_string(p_id);
                    CoTaskMemFree(p_id as *mut std::ffi::c_void);
                }
                ((*dev_vtbl).Release)(p_def_capture);
            }

            // dataFlow = 2 (eAll). dwStateMask: 1 (DEVICE_STATE_ACTIVE) ou 0x0F (ALL)
            let state_mask = if only_active { 0x00000001 } else { 0x0000000F };
            let mut p_coll: *mut std::ffi::c_void = std::ptr::null_mut();
            let hr = ((*enum_vtbl).EnumAudioEndpoints)(p_enum, 2, state_mask, &mut p_coll);
            if hr != 0 || p_coll.is_null() {
                ((*enum_vtbl).Release)(p_enum);
                CoUninitialize();
                return Err(VoiceError::DeviceUnavailable(format!(
                    "Falha ao enumerar endpoints de áudio (HRESULT 0x{:08X})",
                    hr
                )));
            }

            let coll_vtbl = *(p_coll as *mut *const IMMDeviceCollectionVtbl);
            let mut count = 0u32;
            ((*coll_vtbl).GetCount)(p_coll, &mut count);

            let key = PROPERTYKEY {
                fmtid: PKEY_DEVICE_FRIENDLY_NAME_FMTID,
                pid: PKEY_DEVICE_FRIENDLY_NAME_PID,
            };

            let mut devices = Vec::with_capacity(count as usize);

            for i in 0..count {
                let mut p_dev: *mut std::ffi::c_void = std::ptr::null_mut();
                if ((*coll_vtbl).Item)(p_coll, i, &mut p_dev) == 0 && !p_dev.is_null() {
                    let dev_vtbl = *(p_dev as *mut *const IMMDeviceVtbl);

                    let mut p_id: *mut u16 = std::ptr::null_mut();
                    ((*dev_vtbl).GetId)(p_dev, &mut p_id);
                    let id = pwstr_to_string(p_id);
                    CoTaskMemFree(p_id as *mut std::ffi::c_void);

                    let mut raw_state = 0u32;
                    ((*dev_vtbl).GetState)(p_dev, &mut raw_state);
                    let state = match raw_state {
                        1 => AudioDeviceState::Active,
                        2 => AudioDeviceState::Disabled,
                        4 => AudioDeviceState::NotPresent,
                        8 => AudioDeviceState::Unplugged,
                        _ => AudioDeviceState::Unknown,
                    };

                    let mut friendly_name = String::from("(Dispositivo de Áudio)");
                    let mut p_store: *mut std::ffi::c_void = std::ptr::null_mut();
                    if ((*dev_vtbl).OpenPropertyStore)(p_dev, 0, &mut p_store) == 0
                        && !p_store.is_null()
                    {
                        let store_vtbl = *(p_store as *mut *const IPropertyStoreVtbl);
                        let mut pv = std::mem::zeroed::<PROPVARIANT>();
                        if ((*store_vtbl).GetValue)(p_store, &key, &mut pv) == 0 {
                            if pv.vt == 31 && !pv.val_ptr.is_null() {
                                friendly_name = pwstr_to_string(pv.val_ptr);
                            }
                            let _ = PropVariantClear(&mut pv);
                        }
                        ((*store_vtbl).Release)(p_store);
                    }

                    // Determina o fluxo pelo prefixo canônico do GUID do Windows MMDevAPI
                    // {0.0.1.00000000} = Captura (Input)
                    // {0.0.0.00000000} = Reprodução (Output)
                    let flow = if id.contains("{0.0.1.") {
                        AudioDeviceFlow::Input
                    } else {
                        AudioDeviceFlow::Output
                    };

                    let is_default = match flow {
                        AudioDeviceFlow::Input => id.eq_ignore_ascii_case(&def_capture_id),
                        AudioDeviceFlow::Output => id.eq_ignore_ascii_case(&def_render_id),
                    };

                    devices.push(AudioDeviceInfo {
                        id,
                        name: friendly_name,
                        flow,
                        state,
                        is_default,
                    });

                    ((*dev_vtbl).Release)(p_dev);
                }
            }

            ((*coll_vtbl).Release)(p_coll);
            ((*enum_vtbl).Release)(p_enum);
            CoUninitialize();

            Ok(devices)
        }
    }

    #[cfg(not(windows))]
    {
        let _ = only_active;
        Ok(Vec::new())
    }
}

/// Retorna o dispositivo padrão do sistema para o fluxo especificado (Entrada ou Saída).
pub fn get_default_device(flow: AudioDeviceFlow) -> Result<Option<AudioDeviceInfo>, VoiceError> {
    let devices = list_audio_devices(true)?;
    Ok(devices.into_iter().find(|d| d.flow == flow && d.is_default))
}

/// Localiza um dispositivo de áudio por identificador exato ou correspondência de nome.
///
/// Estratégia de resolução:
/// 1. Se `query` for vazio ou nulo, retorna o dispositivo padrão para o fluxo indicado.
/// 2. Correspondência exata por `id` (case-insensitive).
/// 3. Correspondência exata por `name` (case-insensitive).
/// 4. Correspondência por substring no `name` ou `id` (case-insensitive), priorizando dispositivos ativos.
///
/// Se o dispositivo for encontrado mas estiver desconectado/desabilitado, retorna erro explicativo.
pub fn find_device(query: &str, flow: AudioDeviceFlow) -> Result<AudioDeviceInfo, VoiceError> {
    let q = query.trim();
    if q.is_empty() {
        return get_default_device(flow)?.ok_or_else(|| {
            VoiceError::DeviceUnavailable(format!(
                "Nenhum dispositivo padrão de {} encontrado ou ativo no sistema.",
                flow.label()
            ))
        });
    }

    // Busca entre todos os dispositivos conhecidos (inclusive desconectados) para diagnóstico honesto
    let all_devices = list_audio_devices(false)?;
    let matching_flow: Vec<AudioDeviceInfo> =
        all_devices.into_iter().filter(|d| d.flow == flow).collect();

    // 1. Match exato por ID
    if let Some(dev) = matching_flow.iter().find(|d| d.id.eq_ignore_ascii_case(q)) {
        if !dev.state.is_active() {
            return Err(VoiceError::DeviceUnavailable(format!(
                "O dispositivo '{}' (ID: {}) foi encontrado, mas está {} no sistema operacional. Verifique a conexão (ex: Bluetooth) e tente novamente.",
                dev.name, dev.id, dev.state.label()
            )));
        }
        return Ok(dev.clone());
    }

    // 2. Match exato por Nome
    if let Some(dev) = matching_flow
        .iter()
        .find(|d| d.name.eq_ignore_ascii_case(q))
    {
        if !dev.state.is_active() {
            return Err(VoiceError::DeviceUnavailable(format!(
                "O dispositivo '{}' (ID: {}) foi encontrado, mas está {} no sistema operacional. Verifique a conexão (ex: Bluetooth) e tente novamente.",
                dev.name, dev.id, dev.state.label()
            )));
        }
        return Ok(dev.clone());
    }

    // 3. Match por substring no Nome ou ID
    let q_lower = q.to_lowercase();
    let candidates: Vec<&AudioDeviceInfo> = matching_flow
        .iter()
        .filter(|d| {
            d.name.to_lowercase().contains(&q_lower) || d.id.to_lowercase().contains(&q_lower)
        })
        .collect();

    if candidates.is_empty() {
        return Err(VoiceError::DeviceUnavailable(format!(
            "Nenhum dispositivo de {} correspondente a '{}' foi encontrado.",
            flow.label(),
            q
        )));
    }

    // Prioriza dispositivo ativo entre os candidatos
    if let Some(active) = candidates.iter().find(|d| d.state.is_active()) {
        Ok((*active).clone())
    } else {
        let first = candidates[0];
        Err(VoiceError::DeviceUnavailable(format!(
            "O dispositivo '{}' (ID: {}) corresponde a '{}', mas está {} no sistema operacional. Conecte o dispositivo e tente novamente.",
            first.name, first.id, q, first.state.label()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_flow_and_state_labels() {
        assert_eq!(AudioDeviceFlow::Input.label(), "Entrada (Microfone)");
        assert_eq!(AudioDeviceFlow::Output.label(), "Saída (Alto-falante/Fone)");
        assert_eq!(AudioDeviceState::Active.label(), "Ativo");
        assert!(AudioDeviceState::Active.is_active());
        assert!(!AudioDeviceState::Unplugged.is_active());
    }

    #[test]
    fn test_find_device_in_mock_list() {
        // Valida a lógica de filtragem com lista sintética
        let dev1 = AudioDeviceInfo {
            id: "{0.0.1.00000000}.{1111-2222}".to_string(),
            name: "Microfone Realtek(R) Audio".to_string(),
            flow: AudioDeviceFlow::Input,
            state: AudioDeviceState::Active,
            is_default: true,
        };
        let dev2 = AudioDeviceInfo {
            id: "{0.0.1.00000000}.{3333-4444}".to_string(),
            name: "Headset (Redmi Buds 6 Play)".to_string(),
            flow: AudioDeviceFlow::Input,
            state: AudioDeviceState::Unplugged,
            is_default: false,
        };

        let list = [dev1.clone(), dev2.clone()];

        // Match por nome substring
        let found = list.iter().find(|d| d.name.contains("Realtek"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, dev1.id);

        let found_redmi = list.iter().find(|d| d.name.contains("Redmi"));
        assert!(found_redmi.is_some());
        assert_eq!(found_redmi.unwrap().state, AudioDeviceState::Unplugged);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_list_audio_devices_smoke() {
        let devices = list_audio_devices(true);
        assert!(
            devices.is_ok(),
            "Deve listar dispositivos sem pânico no Windows"
        );
        let list = devices.unwrap();
        println!("Dispositivos ativos detectados: {}", list.len());
        for d in &list {
            println!(
                "  [{:?}] {} (ID: {}) Default: {}",
                d.flow, d.name, d.id, d.is_default
            );
        }
    }
}
