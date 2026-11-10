//! Subsistema WASAPI Nativo en Rust (Windows Core Audio COM API):
//! 1. Enumeracion real de todos los dispositivos de salida (`eRender`: audifonos Bluetooth, Jack, USB, altavoces).
//! 2. Captura en tiempo real del audio del navegador/sistema (`IAudioCaptureClient` con `AUDCLNT_STREAMFLAGS_LOOPBACK`).
//! 3. Renderizado directo de baja latencia hacia los audifonos o altavoces (`IAudioRenderClient`).

use std::ffi::c_void;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PropertyKey {
    pub fmtid: Guid,
    pub pid: u32,
}

#[repr(C)]
pub struct PropVariant {
    pub vt: u16,
    pub reservado1: u16,
    pub reservado2: u16,
    pub reservado3: u16,
    pub puntero_valor: *mut u16,
    pub relleno_extra: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct WaveFormatEx {
    pub w_format_tag: u16,
    pub n_channels: u16,
    pub n_samples_per_sec: u32,
    pub n_avg_bytes_per_sec: u32,
    pub n_block_align: u16,
    pub w_bits_per_sample: u16,
    pub cb_size: u16,
}

/// Informacion descriptiva de un dispositivo de salida de audio real de Windows (audifonos o altavoces).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DispositivoAudioWindows {
    pub identificador: String,
    pub nombre_amigable: String,
    pub es_predeterminado: bool,
}

#[cfg(not(target_os = "windows"))]
pub fn enumerar_dispositivos_salida_windows() -> Result<Vec<DispositivoAudioWindows>, String> {
    Ok(Vec::new())
}

#[cfg(not(target_os = "windows"))]
pub struct CapturadorWasapiLoopback {
    pub frecuencia_muestreo: u32,
}

#[cfg(not(target_os = "windows"))]
impl CapturadorWasapiLoopback {
    pub fn inicializar_con_dispositivo(_id: Option<&str>) -> Result<Self, String> {
        Err(String::from("WASAPI solo disponible en Windows"))
    }
    pub fn leer_muestras_estereo(&mut self, _bufer: &mut Vec<(f32, f32)>) -> bool {
        false
    }
}

#[cfg(not(target_os = "windows"))]
pub struct RenderizadorWasapiDirecto;

#[cfg(not(target_os = "windows"))]
impl RenderizadorWasapiDirecto {
    pub fn inicializar_con_dispositivo(_id: Option<&str>) -> Result<Self, String> {
        Err(String::from("WASAPI solo disponible en Windows"))
    }
    pub fn escribir_muestras_estereo(&mut self, _pares: &[(f32, f32)]) -> usize {
        0
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;

    const CLSID_MMDEVICE_ENUMERATOR: Guid = Guid {
    data1: 0xBCDE0395,
    data2: 0xE52F,
    data3: 0x467C,
    data4: [0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E],
};

const IID_IMMDEVICE_ENUMERATOR: Guid = Guid {
    data1: 0xA95664D2,
    data2: 0x9614,
    data3: 0x4F35,
    data4: [0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6],
};

const IID_IAUDIO_CLIENT: Guid = Guid {
    data1: 0x1CB9AD4C,
    data2: 0xDBFA,
    data3: 0x4C32,
    data4: [0xB1, 0x78, 0xC2, 0xF5, 0x68, 0xA7, 0x03, 0xB2],
};

const IID_IAUDIO_CAPTURE_CLIENT: Guid = Guid {
    data1: 0xC8ADBD64,
    data2: 0xE71E,
    data3: 0x48A0,
    data4: [0xA4, 0xDE, 0x18, 0x5C, 0x39, 0x5C, 0xD3, 0x17],
};

const IID_IAUDIO_RENDER_CLIENT: Guid = Guid {
    data1: 0xF294ACFC,
    data2: 0x3146,
    data3: 0x4483,
    data4: [0xA7, 0xBF, 0xAD, 0xDC, 0xA7, 0xC2, 0x60, 0xE2],
};

const PKEY_DEVICE_FRIENDLY_NAME: PropertyKey = PropertyKey {
    fmtid: Guid {
        data1: 0xA45C254E,
        data2: 0xDF1C,
        data3: 0x4EFD,
        data4: [0x80, 0x20, 0x67, 0xD1, 0x46, 0xA8, 0x50, 0xE0],
    },
    pid: 14,
};

const COINIT_MULTITHREADED: u32 = 0x0;
const CLSCTX_ALL: u32 = 23;
const E_RENDER: u32 = 0;
const E_CONSOLE: u32 = 0;
const DEVICE_STATE_ACTIVE: u32 = 0x00000001;
const STGM_READ: u32 = 0x00000000;
const VT_LPWSTR: u16 = 31;
const AUDCLNT_SHAREMODE_SHARED: u32 = 0;
const AUDCLNT_STREAMFLAGS_LOOPBACK: u32 = 0x00020000;
const AUDCLNT_BUFFERFLAGS_SILENT: u32 = 0x2;

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(pv_reserved: *mut c_void, dw_co_init: u32) -> i32;
    fn CoUninitialize();
    fn CoCreateInstance(
        rclsid: *const Guid,
        p_unk_outer: *mut c_void,
        dw_cls_context: u32,
        riid: *const Guid,
        ppv: *mut *mut c_void,
    ) -> i32;
    fn CoTaskMemFree(pv: *mut c_void);
    fn PropVariantClear(pvar: *mut PropVariant) -> i32;
}

#[repr(C)]
struct IMMDeviceEnumeratorVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    enum_audio_endpoints:
        unsafe extern "system" fn(*mut c_void, u32, u32, *mut *mut c_void) -> i32,
    get_default_audio_endpoint:
        unsafe extern "system" fn(*mut c_void, u32, u32, *mut *mut c_void) -> i32,
    get_device: unsafe extern "system" fn(*mut c_void, *const u16, *mut *mut c_void) -> i32,
}

#[repr(C)]
struct IMMDeviceCollectionVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_count: unsafe extern "system" fn(*mut c_void, *mut u32) -> i32,
    item: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32,
}

#[repr(C)]
struct IMMDeviceVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    activate: unsafe extern "system" fn(
        *mut c_void,
        *const Guid,
        u32,
        *const c_void,
        *mut *mut c_void,
    ) -> i32,
    open_property_store: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32,
    get_id: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> i32,
}

#[repr(C)]
struct IPropertyStoreVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_count: usize,
    get_at: usize,
    get_value: unsafe extern "system" fn(*mut c_void, *const PropertyKey, *mut PropVariant) -> i32,
}

#[repr(C)]
struct IAudioClientVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    initialize: unsafe extern "system" fn(
        *mut c_void,
        u32,
        u32,
        i64,
        i64,
        *const WaveFormatEx,
        *const Guid,
    ) -> i32,
    get_buffer_size: unsafe extern "system" fn(*mut c_void, *mut u32) -> i32,
    get_stream_latency: usize,
    get_current_padding: unsafe extern "system" fn(*mut c_void, *mut u32) -> i32,
    is_format_supported: usize,
    get_mix_format: unsafe extern "system" fn(*mut c_void, *mut *mut WaveFormatEx) -> i32,
    get_device_period: usize,
    start: unsafe extern "system" fn(*mut c_void) -> i32,
    stop: unsafe extern "system" fn(*mut c_void) -> i32,
    reset: usize,
    set_event_handle: usize,
    get_service: unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> i32,
}

#[repr(C)]
struct IAudioCaptureClientVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_buffer: unsafe extern "system" fn(
        *mut c_void,
        *mut *mut u8,
        *mut u32,
        *mut u32,
        *mut u64,
        *mut u64,
    ) -> i32,
    release_buffer: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    get_next_packet_size: unsafe extern "system" fn(*mut c_void, *mut u32) -> i32,
}

#[repr(C)]
struct IAudioRenderClientVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_buffer: unsafe extern "system" fn(*mut c_void, u32, *mut *mut u8) -> i32,
    release_buffer: unsafe extern "system" fn(*mut c_void, u32, u32) -> i32,
}

/// Convierte un puntero LPWSTR terminado en nulo de Windows a un `String` UTF-8 seguro.
unsafe fn leer_cadena_utf16(puntero: *const u16) -> String {
    if puntero.is_null() {
        return String::new();
    }
    let mut longitud = 0usize;
    while *puntero.add(longitud) != 0 {
        longitud += 1;
        if longitud > 4096 {
            break;
        }
    }
    let segmento = std::slice::from_raw_parts(puntero, longitud);
    String::from_utf16_lossy(segmento)
}

/// Extrae el ID y el nombre amigable de un `IMMDevice`.
unsafe fn extraer_info_dispositivo(dispositivo: *mut c_void, id_predeterminado: &str) -> Option<DispositivoAudioWindows> {
    if dispositivo.is_null() {
        return None;
    }
    let vtbl_dev = *(dispositivo as *const *const IMMDeviceVtbl);

    let mut puntero_id: *mut u16 = ptr::null_mut();
    if ((*vtbl_dev).get_id)(dispositivo, &mut puntero_id) < 0 || puntero_id.is_null() {
        return None;
    }
    let identificador = leer_cadena_utf16(puntero_id);
    CoTaskMemFree(puntero_id as *mut c_void);

    let mut nombre_amigable = String::from("Dispositivo de Salida de Audio");
    let mut almacen_propiedades: *mut c_void = ptr::null_mut();
    if ((*vtbl_dev).open_property_store)(dispositivo, STGM_READ, &mut almacen_propiedades) >= 0
        && !almacen_propiedades.is_null()
    {
        let vtbl_prop = *(almacen_propiedades as *const *const IPropertyStoreVtbl);
        let mut variante = PropVariant {
            vt: 0,
            reservado1: 0,
            reservado2: 0,
            reservado3: 0,
            puntero_valor: ptr::null_mut(),
            relleno_extra: 0,
        };
        if ((*vtbl_prop).get_value)(
            almacen_propiedades,
            &PKEY_DEVICE_FRIENDLY_NAME,
            &mut variante,
        ) >= 0
        {
            if variante.vt == VT_LPWSTR && !variante.puntero_valor.is_null() {
                let leido = leer_cadena_utf16(variante.puntero_valor);
                if !leido.is_empty() {
                    nombre_amigable = leido;
                }
            }
            let _ = PropVariantClear(&mut variante);
        }
        ((*vtbl_prop).release)(almacen_propiedades);
    }

    let es_predeterminado = !id_predeterminado.is_empty() && identificador == id_predeterminado;
    Some(DispositivoAudioWindows {
        identificador,
        nombre_amigable,
        es_predeterminado,
    })
}

/// Enumera todos los dispositivos de salida de audio (`eRender`) activos en Windows (audifonos, altavoces, HDMI, DACs).
pub fn enumerar_dispositivos_salida_windows() -> Result<Vec<DispositivoAudioWindows>, String> {
    // Se ejecuta en un hilo aislado para no alterar el modo del apartamento COM del hilo principal (UI winit STA)
    std::thread::spawn(|| {
        unsafe {
            let _ = CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED);
            let resultado = enumerar_dispositivos_salida_windows_interno();
            CoUninitialize();
            resultado
        }
    })
    .join()
    .unwrap_or_else(|_| Err(String::from("Fallo al unir hilo de enumeracion de dispositivos")))
}

unsafe fn enumerar_dispositivos_salida_windows_interno() -> Result<Vec<DispositivoAudioWindows>, String> {
    let mut enumerador: *mut c_void = ptr::null_mut();
    let hr_enum = CoCreateInstance(
        &CLSID_MMDEVICE_ENUMERATOR,
        ptr::null_mut(),
        CLSCTX_ALL,
        &IID_IMMDEVICE_ENUMERATOR,
        &mut enumerador,
    );
    if hr_enum < 0 || enumerador.is_null() {
        return Err(format!(
            "No se pudo inicializar IMMDeviceEnumerator (0x{:08X})",
            hr_enum
        ));
    }

        let vtbl_enum = *(enumerador as *const *const IMMDeviceEnumeratorVtbl);

        // Obtener ID del dispositivo predeterminado actual
        let mut id_predeterminado = String::new();
        let mut disp_def: *mut c_void = ptr::null_mut();
        if ((*vtbl_enum).get_default_audio_endpoint)(enumerador, E_RENDER, E_CONSOLE, &mut disp_def) >= 0
            && !disp_def.is_null()
        {
            let vtbl_d = *(disp_def as *const *const IMMDeviceVtbl);
            let mut ptr_id: *mut u16 = ptr::null_mut();
            if ((*vtbl_d).get_id)(disp_def, &mut ptr_id) >= 0 && !ptr_id.is_null() {
                id_predeterminado = leer_cadena_utf16(ptr_id);
                CoTaskMemFree(ptr_id as *mut c_void);
            }
            ((*vtbl_d).release)(disp_def);
        }

        let mut coleccion: *mut c_void = ptr::null_mut();
        let hr_col = ((*vtbl_enum).enum_audio_endpoints)(
            enumerador,
            E_RENDER,
            DEVICE_STATE_ACTIVE,
            &mut coleccion,
        );
        ((*vtbl_enum).release)(enumerador);

        if hr_col < 0 || coleccion.is_null() {
            return Err(format!(
                "No se pudo enumerar endpoints de salida (0x{:08X})",
                hr_col
            ));
        }

        let vtbl_col = *(coleccion as *const *const IMMDeviceCollectionVtbl);
        let mut cantidad: u32 = 0;
        let _ = ((*vtbl_col).get_count)(coleccion, &mut cantidad);

        let mut lista = Vec::with_capacity(cantidad as usize);
        for indice in 0..cantidad {
            let mut disp: *mut c_void = ptr::null_mut();
            if ((*vtbl_col).item)(coleccion, indice, &mut disp) >= 0 && !disp.is_null() {
                if let Some(info) = extraer_info_dispositivo(disp, &id_predeterminado) {
                    lista.push(info);
                }
                let vtbl_d = *(disp as *const *const IMMDeviceVtbl);
                ((*vtbl_d).release)(disp);
            }
        }

        ((*vtbl_col).release)(coleccion);
        Ok(lista)
}

/// Obtiene un puntero `IMMDevice` por su identificador o el predeterminado de salida (`eRender`).
unsafe fn abrir_mmdevice_por_id_o_predeterminado(
    id_dispositivo: Option<&str>,
) -> Result<(*mut c_void, DispositivoAudioWindows), String> {
    let _ = CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED);

    let mut enumerador: *mut c_void = ptr::null_mut();
    let hr_enum = CoCreateInstance(
        &CLSID_MMDEVICE_ENUMERATOR,
        ptr::null_mut(),
        CLSCTX_ALL,
        &IID_IMMDEVICE_ENUMERATOR,
        &mut enumerador,
    );
    if hr_enum < 0 || enumerador.is_null() {
        return Err(format!(
            "Fallo al crear IMMDeviceEnumerator (0x{:08X})",
            hr_enum
        ));
    }

    let vtbl_enum = *(enumerador as *const *const IMMDeviceEnumeratorVtbl);
    let mut dispositivo: *mut c_void = ptr::null_mut();

    let hr_dev = match id_dispositivo.filter(|s| !s.is_empty() && *s != "predeterminado") {
        Some(id_str) => {
            let mut utf16: Vec<u16> = id_str.encode_utf16().collect();
            utf16.push(0);
            ((*vtbl_enum).get_device)(enumerador, utf16.as_ptr(), &mut dispositivo)
        }
        None => ((*vtbl_enum).get_default_audio_endpoint)(
            enumerador,
            E_RENDER,
            E_CONSOLE,
            &mut dispositivo,
        ),
    };
    ((*vtbl_enum).release)(enumerador);

    if hr_dev < 0 || dispositivo.is_null() {
        return Err(format!(
            "No se encontro el dispositivo de salida solicitado (0x{:08X})",
            hr_dev
        ));
    }

    let info = extraer_info_dispositivo(dispositivo, "").unwrap_or(DispositivoAudioWindows {
        identificador: String::from("predeterminado"),
        nombre_amigable: String::from("Salida Predeterminada de Windows"),
        es_predeterminado: true,
    });

    Ok((dispositivo, info))
}

/// Capturador WASAPI Loopback de un endpoint de salida (navegador / sistema).
pub struct CapturadorWasapiLoopback {
    cliente_audio: *mut c_void,
    cliente_captura: *mut c_void,
    pub info_dispositivo: DispositivoAudioWindows,
    pub frecuencia_muestreo: u32,
    pub canales: u16,
    pub bits_por_muestra: u16,
}

unsafe impl Send for CapturadorWasapiLoopback {}

impl CapturadorWasapiLoopback {
    /// Inicializa la captura WASAPI Loopback sobre la salida predeterminada de Windows.
    pub fn inicializar() -> Result<Self, String> {
        Self::inicializar_con_dispositivo(None)
    }

    /// Inicializa la captura WASAPI Loopback sobre un dispositivo especifico o el predeterminado (`None`).
    pub fn inicializar_con_dispositivo(id_dispositivo: Option<&str>) -> Result<Self, String> {
        unsafe {
            let (dispositivo, info_dispositivo) =
                abrir_mmdevice_por_id_o_predeterminado(id_dispositivo)?;

            let vtbl_dev = *(dispositivo as *const *const IMMDeviceVtbl);
            let mut cliente_audio: *mut c_void = ptr::null_mut();
            let hr_act = ((*vtbl_dev).activate)(
                dispositivo,
                &IID_IAUDIO_CLIENT,
                CLSCTX_ALL,
                ptr::null(),
                &mut cliente_audio,
            );
            ((*vtbl_dev).release)(dispositivo);

            if hr_act < 0 || cliente_audio.is_null() {
                return Err(format!(
                    "Fallo al activar IAudioClient para captura (0x{:08X})",
                    hr_act
                ));
            }

            let vtbl_client = *(cliente_audio as *const *const IAudioClientVtbl);
            let mut formato_mezcla: *mut WaveFormatEx = ptr::null_mut();
            let hr_fmt = ((*vtbl_client).get_mix_format)(cliente_audio, &mut formato_mezcla);
            if hr_fmt < 0 || formato_mezcla.is_null() {
                ((*vtbl_client).release)(cliente_audio);
                return Err(format!(
                    "Fallo al obtener MixFormat WASAPI (0x{:08X})",
                    hr_fmt
                ));
            }

            let canales = (*formato_mezcla).n_channels;
            let frecuencia_muestreo = (*formato_mezcla).n_samples_per_sec;
            let bits_por_muestra = (*formato_mezcla).w_bits_per_sample;

            // Bufer de 20 ms en modo Loopback compartido
            let duracion_hns: i64 = 200_000;
            let hr_init = ((*vtbl_client).initialize)(
                cliente_audio,
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                duracion_hns,
                0,
                formato_mezcla,
                ptr::null(),
            );
            CoTaskMemFree(formato_mezcla as *mut c_void);

            if hr_init < 0 {
                ((*vtbl_client).release)(cliente_audio);
                return Err(format!(
                    "Fallo al inicializar IAudioClient en modo Loopback (0x{:08X})",
                    hr_init
                ));
            }

            let mut cliente_captura: *mut c_void = ptr::null_mut();
            let hr_cap = ((*vtbl_client).get_service)(
                cliente_audio,
                &IID_IAUDIO_CAPTURE_CLIENT,
                &mut cliente_captura,
            );
            if hr_cap < 0 || cliente_captura.is_null() {
                ((*vtbl_client).release)(cliente_audio);
                return Err(format!(
                    "Fallo al obtener IAudioCaptureClient (0x{:08X})",
                    hr_cap
                ));
            }

            let hr_start = ((*vtbl_client).start)(cliente_audio);
            if hr_start < 0 {
                let vtbl_cap = *(cliente_captura as *const *const IAudioCaptureClientVtbl);
                ((*vtbl_cap).release)(cliente_captura);
                ((*vtbl_client).release)(cliente_audio);
                return Err(format!(
                    "Fallo al iniciar flujo WASAPI Loopback (0x{:08X})",
                    hr_start
                ));
            }

            Ok(Self {
                cliente_audio,
                cliente_captura,
                info_dispositivo,
                frecuencia_muestreo,
                canales,
                bits_por_muestra,
            })
        }
    }

    /// Extrae todos los fotogramas estereo disponibles en el bufer de salida de Windows (navegador/sistema).
    pub fn leer_muestras_estereo(&mut self, destino_pares: &mut Vec<(f32, f32)>) -> bool {
        destino_pares.clear();
        let mut leyo_algo = false;

        unsafe {
            let vtbl_cap = *(self.cliente_captura as *const *const IAudioCaptureClientVtbl);
            let mut tamano_paquete: u32 = 0;

            if ((*vtbl_cap).get_next_packet_size)(self.cliente_captura, &mut tamano_paquete) < 0 {
                return false;
            }

            while tamano_paquete > 0 {
                let mut puntero_datos: *mut u8 = ptr::null_mut();
                let mut fotogramas_a_leer: u32 = 0;
                let mut banderas: u32 = 0;

                let hr = ((*vtbl_cap).get_buffer)(
                    self.cliente_captura,
                    &mut puntero_datos,
                    &mut fotogramas_a_leer,
                    &mut banderas,
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                if hr < 0 {
                    break;
                }

                let es_silencio =
                    (banderas & AUDCLNT_BUFFERFLAGS_SILENT) != 0 || puntero_datos.is_null();
                let n_canales = (self.canales as usize).max(1);

                if es_silencio {
                    for _ in 0..fotogramas_a_leer {
                        destino_pares.push((0.0, 0.0));
                    }
                } else if self.bits_por_muestra == 32 {
                    let muestras_f32 = std::slice::from_raw_parts(
                        puntero_datos as *const f32,
                        (fotogramas_a_leer as usize) * n_canales,
                    );
                    for cuadro in muestras_f32.chunks_exact(n_canales) {
                        let izq = cuadro[0];
                        let der = if n_canales > 1 { cuadro[1] } else { izq };
                        destino_pares.push((izq, der));
                    }
                } else if self.bits_por_muestra == 16 {
                    let muestras_i16 = std::slice::from_raw_parts(
                        puntero_datos as *const i16,
                        (fotogramas_a_leer as usize) * n_canales,
                    );
                    for cuadro in muestras_i16.chunks_exact(n_canales) {
                        let izq = (cuadro[0] as f32) / 32768.0;
                        let der = if n_canales > 1 {
                            (cuadro[1] as f32) / 32768.0
                        } else {
                            izq
                        };
                        destino_pares.push((izq, der));
                    }
                }

                let _ = ((*vtbl_cap).release_buffer)(self.cliente_captura, fotogramas_a_leer);
                leyo_algo = true;

                if ((*vtbl_cap).get_next_packet_size)(self.cliente_captura, &mut tamano_paquete) < 0
                {
                    break;
                }
            }
        }

        leyo_algo
    }
}

impl Drop for CapturadorWasapiLoopback {
    fn drop(&mut self) {
        unsafe {
            if !self.cliente_captura.is_null() {
                let vtbl_cap = *(self.cliente_captura as *const *const IAudioCaptureClientVtbl);
                ((*vtbl_cap).release)(self.cliente_captura);
            }
            if !self.cliente_audio.is_null() {
                let vtbl_client = *(self.cliente_audio as *const *const IAudioClientVtbl);
                let _ = ((*vtbl_client).stop)(self.cliente_audio);
                ((*vtbl_client).release)(self.cliente_audio);
            }
        }
    }
}

/// Renderizador WASAPI Directo (`IAudioRenderClient`) que envia el audio procesado por la CadenaMaestraDsp
/// directamente hacia los audifonos o altavoces seleccionados en Windows.
pub struct RenderizadorWasapiDirecto {
    cliente_audio: *mut c_void,
    cliente_render: *mut c_void,
    pub info_dispositivo: DispositivoAudioWindows,
    pub frecuencia_muestreo: u32,
    pub canales: u16,
    pub bits_por_muestra: u16,
    pub tamano_bufer_fotogramas: u32,
}

unsafe impl Send for RenderizadorWasapiDirecto {}

impl RenderizadorWasapiDirecto {
    /// Inicializa un flujo de salida WASAPI directo hacia el dispositivo indicado (o el predeterminado).
    pub fn inicializar_con_dispositivo(id_dispositivo: Option<&str>) -> Result<Self, String> {
        unsafe {
            let (dispositivo, info_dispositivo) =
                abrir_mmdevice_por_id_o_predeterminado(id_dispositivo)?;

            let vtbl_dev = *(dispositivo as *const *const IMMDeviceVtbl);
            let mut cliente_audio: *mut c_void = ptr::null_mut();
            let hr_act = ((*vtbl_dev).activate)(
                dispositivo,
                &IID_IAUDIO_CLIENT,
                CLSCTX_ALL,
                ptr::null(),
                &mut cliente_audio,
            );
            ((*vtbl_dev).release)(dispositivo);

            if hr_act < 0 || cliente_audio.is_null() {
                return Err(format!(
                    "Fallo al activar IAudioClient de salida (0x{:08X})",
                    hr_act
                ));
            }

            let vtbl_client = *(cliente_audio as *const *const IAudioClientVtbl);
            let mut formato_mezcla: *mut WaveFormatEx = ptr::null_mut();
            let hr_fmt = ((*vtbl_client).get_mix_format)(cliente_audio, &mut formato_mezcla);
            if hr_fmt < 0 || formato_mezcla.is_null() {
                ((*vtbl_client).release)(cliente_audio);
                return Err(format!(
                    "Fallo al obtener MixFormat de salida (0x{:08X})",
                    hr_fmt
                ));
            }

            let canales = (*formato_mezcla).n_channels;
            let frecuencia_muestreo = (*formato_mezcla).n_samples_per_sec;
            let bits_por_muestra = (*formato_mezcla).w_bits_per_sample;

            // Bufer de 25 ms de baja latencia para salida hacia audifonos / altavoces
            let duracion_hns: i64 = 250_000;
            let hr_init = ((*vtbl_client).initialize)(
                cliente_audio,
                AUDCLNT_SHAREMODE_SHARED,
                0,
                duracion_hns,
                0,
                formato_mezcla,
                ptr::null(),
            );
            CoTaskMemFree(formato_mezcla as *mut c_void);

            if hr_init < 0 {
                ((*vtbl_client).release)(cliente_audio);
                return Err(format!(
                    "Fallo al inicializar IAudioClient de salida (0x{:08X})",
                    hr_init
                ));
            }

            let mut tamano_bufer_fotogramas: u32 = 0;
            let _ = ((*vtbl_client).get_buffer_size)(cliente_audio, &mut tamano_bufer_fotogramas);

            let mut cliente_render: *mut c_void = ptr::null_mut();
            let hr_ren = ((*vtbl_client).get_service)(
                cliente_audio,
                &IID_IAUDIO_RENDER_CLIENT,
                &mut cliente_render,
            );
            if hr_ren < 0 || cliente_render.is_null() {
                ((*vtbl_client).release)(cliente_audio);
                return Err(format!(
                    "Fallo al obtener IAudioRenderClient (0x{:08X})",
                    hr_ren
                ));
            }

            let hr_start = ((*vtbl_client).start)(cliente_audio);
            if hr_start < 0 {
                let vtbl_ren = *(cliente_render as *const *const IAudioRenderClientVtbl);
                ((*vtbl_ren).release)(cliente_render);
                ((*vtbl_client).release)(cliente_audio);
                return Err(format!(
                    "Fallo al iniciar reproduccion WASAPI (0x{:08X})",
                    hr_start
                ));
            }

            Ok(Self {
                cliente_audio,
                cliente_render,
                info_dispositivo,
                frecuencia_muestreo,
                canales,
                bits_por_muestra,
                tamano_bufer_fotogramas,
            })
        }
    }

    /// Escribe los fotogramas estereo ya procesados por la `CadenaMaestraDsp` hacia el bufer de salida de Windows.
    pub fn escribir_muestras_estereo(&mut self, pares_procesados: &[(f32, f32)]) -> usize {
        if pares_procesados.is_empty() {
            return 0;
        }

        unsafe {
            let vtbl_client = *(self.cliente_audio as *const *const IAudioClientVtbl);
            let vtbl_ren = *(self.cliente_render as *const *const IAudioRenderClientVtbl);

            let mut relleno_actual: u32 = 0;
            if ((*vtbl_client).get_current_padding)(self.cliente_audio, &mut relleno_actual) < 0 {
                return 0;
            }

            let disponibles = self.tamano_bufer_fotogramas.saturating_sub(relleno_actual) as usize;
            let a_escribir = pares_procesados.len().min(disponibles);
            if a_escribir == 0 {
                return 0;
            }

            let mut puntero_salida: *mut u8 = ptr::null_mut();
            if ((*vtbl_ren).get_buffer)(self.cliente_render, a_escribir as u32, &mut puntero_salida)
                < 0
                || puntero_salida.is_null()
            {
                return 0;
            }

            let n_canales = (self.canales as usize).max(1);
            if self.bits_por_muestra == 32 {
                let destino_f32 = std::slice::from_raw_parts_mut(
                    puntero_salida as *mut f32,
                    a_escribir * n_canales,
                );
                for (i, &(izq, der)) in pares_procesados[..a_escribir].iter().enumerate() {
                    let base = i * n_canales;
                    destino_f32[base] = izq;
                    if n_canales > 1 {
                        destino_f32[base + 1] = der;
                    }
                    for c in 2..n_canales {
                        destino_f32[base + c] = 0.0;
                    }
                }
            } else if self.bits_por_muestra == 16 {
                let destino_i16 = std::slice::from_raw_parts_mut(
                    puntero_salida as *mut i16,
                    a_escribir * n_canales,
                );
                for (i, &(izq, der)) in pares_procesados[..a_escribir].iter().enumerate() {
                    let base = i * n_canales;
                    destino_i16[base] = (izq.clamp(-1.0, 1.0) * 32767.0) as i16;
                    if n_canales > 1 {
                        destino_i16[base + 1] = (der.clamp(-1.0, 1.0) * 32767.0) as i16;
                    }
                    for c in 2..n_canales {
                        destino_i16[base + c] = 0;
                    }
                }
            }

            let _ = ((*vtbl_ren).release_buffer)(self.cliente_render, a_escribir as u32, 0);
            a_escribir
        }
    }
}

impl Drop for RenderizadorWasapiDirecto {
    fn drop(&mut self) {
        unsafe {
            if !self.cliente_render.is_null() {
                let vtbl_ren = *(self.cliente_render as *const *const IAudioRenderClientVtbl);
                ((*vtbl_ren).release)(self.cliente_render);
            }
            if !self.cliente_audio.is_null() {
                let vtbl_client = *(self.cliente_audio as *const *const IAudioClientVtbl);
                let _ = ((*vtbl_client).stop)(self.cliente_audio);
                ((*vtbl_client).release)(self.cliente_audio);
            }
        }
    }
}

}
#[cfg(target_os = "windows")]
pub use windows_impl::*;
