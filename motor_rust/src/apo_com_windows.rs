//! Implementacion Nativa en Rust de nuestro propio Windows Audio Processing Object (APO COM DLL)
//! y Sincronizacion IPC Binaria de Estado (`estado_dsp_activo.bin`) entre la Interfaz y el Motor de Sistema.
//!
//! Cuando se compila como `cdylib` (`motor_ecualizador_poweramp.dll`), este modulo expone:
//! - `DllGetClassObject`, `DllCanUnloadNow`, `DllRegisterServer`, `DllUnregisterServer`
//! - Las tablas virtuales COM de `IAudioProcessingObject`, `IAudioProcessingObjectRT`,
//!   `IAudioProcessingObjectConfiguration`, `IAudioSystemEffects` e `IClassFactory`.
//! - Procesamiento en sitio (`APOProcess`) a 64 bits sin depender de Equalizer APO ni software externo.

use std::ffi::c_void;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::cadena_maestra_dsp::CadenaMaestraDsp;
use crate::captura_wasapi_loopback::Guid;
use crate::tipos_audio::{
    BandaParametrica, CanalAplicacion, EstadoCompletoDsp, ParametrosCompresor, ParametrosEspacio,
    ParametrosReverberacion, ParametrosTono, TipoFiltro,
};

/// CLSID soberano de nuestro APO COM en Rust: `{8A7B9C10-1706-4A1D-9E2B-A1D2A3F4E501}`
pub const CLSID_APO_SOBERANO_RUST: Guid = Guid {
    data1: 0x8A7B9C10,
    data2: 0x1706,
    data3: 0x4A1D,
    data4: [0x9E, 0x2B, 0xA1, 0xD2, 0xA3, 0xF4, 0xE5, 0x01],
};

pub const CLSID_APO_SOBERANO_TEXTO: &str = "{8A7B9C10-1706-4A1D-9E2B-A1D2A3F4E501}";

const IID_IUNKNOWN: Guid = Guid {
    data1: 0x00000000,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

#[allow(dead_code)]
const IID_ICLASS_FACTORY: Guid = Guid {
    data1: 0x00000001,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const IID_IAUDIO_PROCESSING_OBJECT: Guid = Guid {
    data1: 0xFD7F2B29,
    data2: 0x24D0,
    data3: 0x4B5C,
    data4: [0xB1, 0x77, 0x59, 0x2C, 0x39, 0xF9, 0xCA, 0x10],
};

const IID_IAUDIO_PROCESSING_OBJECT_RT: Guid = Guid {
    data1: 0x9E1D6A6D,
    data2: 0xDDBC,
    data3: 0x4E95,
    data4: [0xA4, 0xC7, 0xAD, 0x64, 0xBA, 0x37, 0x84, 0x6C],
};

const IID_IAUDIO_PROCESSING_OBJECT_CONFIG: Guid = Guid {
    data1: 0x0E5ED805,
    data2: 0xABA6,
    data3: 0x49C3,
    data4: [0x8F, 0x9A, 0x2B, 0x8C, 0x88, 0x9C, 0x4F, 0xA8],
};

const IID_IAUDIO_SYSTEM_EFFECTS: Guid = Guid {
    data1: 0x5FA00F27,
    data2: 0xADD6,
    data3: 0x499A,
    data4: [0x8A, 0x9D, 0x6B, 0x98, 0x52, 0x1F, 0xA7, 0x5B],
};

const S_OK: i32 = 0;
#[allow(dead_code)]
const S_FALSE: i32 = 1;
const E_NOINTERFACE: i32 = -2147467262; // 0x80004002
const E_POINTER: i32 = -2147467261; // 0x80004003
#[allow(dead_code)]
const CLASS_E_NOAGGREGATION: i32 = -2147221232; // 0x80040110
#[allow(dead_code)]
const CLASS_E_CLASSNOTAVAILABLE: i32 = -2147221231; // 0x80040111

// Valores reales de `APO_BUFFER_FLAGS` (audioenginebaseapo.h): INVALID = 0, VALID = 1, SILENT = 2.
const BUFFER_VALID: u32 = 1;
const BUFFER_SILENT: u32 = 2;

/// `APOERR_FORMAT_NOT_SUPPORTED` (0x887D0003).
const APOERR_FORMAT_NOT_SUPPORTED: i32 = 0x887D0003_u32 as i32;
const E_OUTOFMEMORY: i32 = 0x8007000E_u32 as i32;

/// `APO_FLAG_INPLACE | APO_FLAG_DEFAULT` (procesamiento en sitio + formatos iguales entrada/salida).
pub const BANDERAS_APO_REGISTRO: u32 = 0x0000000F;
pub const NOMBRE_AMIGABLE_APO: &str = "Aldra Poweramp DSP (Rust)";
pub const DERECHOS_APO: &str = "Jose Aldra Aviles Cardenas";

const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;

#[cfg(target_os = "windows")]
#[link(name = "ole32")]
extern "system" {
    fn CoTaskMemAlloc(cb: usize) -> *mut c_void;
}

#[cfg(not(target_os = "windows"))]
#[allow(non_snake_case)]
unsafe fn CoTaskMemAlloc(cb: usize) -> *mut c_void {
    let disposicion = std::alloc::Layout::from_size_align(cb, 8).unwrap_or(std::alloc::Layout::new::<u8>());
    std::alloc::alloc(disposicion) as *mut c_void
}

/// `WAVEFORMATEX` empaquetado (tal como lo entrega `IAudioMediaType::GetAudioFormat`).
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct FormatoOndaEx {
    etiqueta_formato: u16,
    canales: u16,
    muestras_por_segundo: u32,
    bytes_promedio_por_segundo: u32,
    alineacion_bloque: u16,
    bits_por_muestra: u16,
    tamano_extra: u16,
}

/// Tabla virtual de `IAudioMediaType`.
#[repr(C)]
struct IAudioMediaTypeVtbl {
    query_interface: usize,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: usize,
    is_compressed_format: usize,
    is_equal: usize,
    get_audio_format: unsafe extern "system" fn(*mut c_void) -> *const FormatoOndaEx,
    get_uncompressed_audio_format: usize,
}

/// `APO_CONNECTION_DESCRIPTOR` (x64).
#[repr(C)]
struct DescriptorConexionApo {
    tipo: u32,
    p_buffer: usize,
    max_fotogramas: u32,
    p_formato: *mut c_void,
    firma: u32,
}

/// `APO_REG_PROPERTIES` con una sola interfaz declarada.
#[repr(C)]
struct PropiedadesRegistroApo {
    clsid: Guid,
    banderas: u32,
    nombre_amigable: [u16; 256],
    derechos: [u16; 256],
    version_mayor: u32,
    version_menor: u32,
    min_conexiones_entrada: u32,
    max_conexiones_entrada: u32,
    min_conexiones_salida: u32,
    max_conexiones_salida: u32,
    max_instancias: u32,
    numero_interfaces: u32,
    lista_interfaces: [Guid; 1],
}

/// Lee el `WAVEFORMATEX` de un `IAudioMediaType` y devuelve (canales, frecuencia, es_float32).
unsafe fn leer_formato_tipo_medio(tipo_medio: *mut c_void) -> Option<(u16, u32, bool)> {
    if tipo_medio.is_null() {
        return None;
    }
    let vtbl = *(tipo_medio as *const *const IAudioMediaTypeVtbl);
    let formato = ((*vtbl).get_audio_format)(tipo_medio);
    if formato.is_null() {
        return None;
    }
    let f = *formato;
    let etiqueta = f.etiqueta_formato;
    let bits = f.bits_por_muestra;
    let es_float = bits == 32 && (etiqueta == WAVE_FORMAT_IEEE_FLOAT || etiqueta == WAVE_FORMAT_EXTENSIBLE);
    Some((f.canales, f.muestras_por_segundo, es_float))
}

fn copiar_texto_utf16(destino: &mut [u16; 256], texto: &str) {
    for (i, unidad) in texto.encode_utf16().take(255).enumerate() {
        destino[i] = unidad;
    }
}

static OBJETOS_COM_ACTIVOS: AtomicU32 = AtomicU32::new(0);
static CONTADOR_VERSION_IPC: AtomicU64 = AtomicU64::new(1);

/// Propiedad de conexion de bufer de tiempo real que `audiodg.exe` entrega a `APOProcess`.
#[repr(C)]
pub struct ApoConnectionProperty {
    pub p_buffer: usize,
    pub u32_valid_frame_count: u32,
    pub u32_buffer_flags: u32,
    pub u32_signature: u32,
}

/// Cabecera binaria determinista `APOR\x03` para compartir el estado DSP entre la UI y el APO COM.
const MAGIA_IPC_APO: &[u8; 4] = b"APO3";

/// Devuelve la ruta compartida accesible tanto por el usuario como por `LOCAL SERVICE` (`audiodg.exe`).
pub fn ruta_estado_ipc_global(directorio_base_opcional: Option<&Path>) -> PathBuf {
    let dir_program_data = Path::new(r"C:\ProgramData\AldraPowerampDsp");
    if dir_program_data.exists() || fs::create_dir_all(dir_program_data).is_ok() {
        return dir_program_data.join("estado_dsp_activo.bin");
    }
    if let Some(base) = directorio_base_opcional {
        return base.join("estado_dsp_activo.bin");
    }
    PathBuf::from("estado_dsp_activo.bin")
}

/// Serializa el `EstadoCompletoDsp` a un bloque binario compacto y determinista para IPC en tiempo real.
pub fn guardar_estado_binario_ipc(ruta: &Path, estado: &EstadoCompletoDsp) -> Result<(), String> {
    let base_tiempo = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    // Version monotona unica entre reinicios de la interfaz (marca de tiempo + contador)
    let version = base_tiempo.wrapping_add(CONTADOR_VERSION_IPC.fetch_add(1, Ordering::Relaxed));
    let mut bytes = Vec::with_capacity(512);

    bytes.extend_from_slice(MAGIA_IPC_APO);
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.push(if estado.ecualizador_activo { 1 } else { 0 });
    bytes.push(if estado.limitador_activo { 1 } else { 0 });
    bytes.push(if estado.tono.tono_activo { 1 } else { 0 });
    bytes.push(if estado.espacio.modo_mono { 1 } else { 0 });
    bytes.push(if estado.compresor.activo { 1 } else { 0 });
    bytes.push(if estado.reverberacion.activo { 1 } else { 0 });

    let num_bandas = (estado.bandas.len().min(32)) as u16;
    bytes.extend_from_slice(&num_bandas.to_le_bytes());

    // Preamp + Tono
    bytes.extend_from_slice(&estado.ganancia_preamp_db.to_le_bytes());
    bytes.extend_from_slice(&estado.tono.graves_ganancia_db.to_le_bytes());
    bytes.extend_from_slice(&estado.tono.graves_frecuencia_hz.to_le_bytes());
    bytes.extend_from_slice(&estado.tono.graves_factor_q.to_le_bytes());
    bytes.extend_from_slice(&estado.tono.agudos_ganancia_db.to_le_bytes());
    bytes.extend_from_slice(&estado.tono.agudos_frecuencia_hz.to_le_bytes());
    bytes.extend_from_slice(&estado.tono.agudos_factor_q.to_le_bytes());

    // Espacio
    bytes.extend_from_slice(&estado.espacio.balance.to_le_bytes());
    bytes.extend_from_slice(&estado.espacio.expansion_estereo.to_le_bytes());

    // Compresor
    bytes.extend_from_slice(&estado.compresor.limite_db.to_le_bytes());
    bytes.extend_from_slice(&estado.compresor.proporcion.to_le_bytes());
    bytes.extend_from_slice(&estado.compresor.ataque_ms.to_le_bytes());
    bytes.extend_from_slice(&estado.compresor.lanzamiento_ms.to_le_bytes());
    bytes.extend_from_slice(&estado.compresor.frecuencia_sidechain_hz.to_le_bytes());
    bytes.extend_from_slice(&estado.compresor.ganancia_maquillaje_db.to_le_bytes());

    // Reverberacion
    bytes.extend_from_slice(&estado.reverberacion.tamano.to_le_bytes());
    bytes.extend_from_slice(&estado.reverberacion.preretardo_ms.to_le_bytes());
    bytes.extend_from_slice(&estado.reverberacion.amortiguacion.to_le_bytes());
    bytes.extend_from_slice(&estado.reverberacion.filtro_tonal.to_le_bytes());
    bytes.extend_from_slice(&estado.reverberacion.mezclado.to_le_bytes());

    // Bandas parametricas (cada banda = 28 bytes)
    for b in estado.bandas.iter().take(num_bandas as usize) {
        bytes.extend_from_slice(&b.frecuencia_hz.to_le_bytes());
        bytes.extend_from_slice(&b.factor_q.to_le_bytes());
        bytes.extend_from_slice(&b.ganancia_db.to_le_bytes());
        let codigo_tipo: u8 = match b.tipo {
            TipoFiltro::PicoParametrico => 0,
            TipoFiltro::EstanteGraves => 1,
            TipoFiltro::EstanteAgudos => 2,
            TipoFiltro::PasoBajo => 3,
            TipoFiltro::PasoAlto => 4,
            TipoFiltro::PasoBanda => 5,
        };
        bytes.push(codigo_tipo);
        bytes.push(if b.activo { 1 } else { 0 });
        bytes.extend_from_slice(&[0u8; 2]);
    }

    // Escritura atomica (temporal + renombrado) para que el APO nunca lea un bloque a medias
    let ruta_temporal = ruta.with_extension("tmp");
    fs::write(&ruta_temporal, &bytes).map_err(|e| format!("Error al escribir IPC binario: {}", e))?;
    if fs::rename(&ruta_temporal, ruta).is_err() {
        fs::write(ruta, &bytes).map_err(|e| format!("Error al escribir IPC binario: {}", e))?;
    }
    Ok(())
}

/// Deserializa el bloque binario IPC si su version es mas reciente que `ultima_version`.
pub fn cargar_estado_binario_ipc_si_cambio(
    ruta: &Path,
    ultima_version: &mut u64,
) -> Option<EstadoCompletoDsp> {
    let bytes = fs::read(ruta).ok()?;
    if bytes.len() < 180 || &bytes[0..4] != MAGIA_IPC_APO {
        return None;
    }

    let version = u64::from_le_bytes(bytes[4..12].try_into().ok()?);
    if version == *ultima_version {
        return None;
    }
    *ultima_version = version;

    let ecualizador_activo = bytes[12] != 0;
    let limitador_activo = bytes[13] != 0;
    let tono_activo = bytes[14] != 0;
    let modo_mono = bytes[15] != 0;
    let compresor_activo = bytes[16] != 0;
    let reverb_activo = bytes[17] != 0;
    let num_bandas = u16::from_le_bytes([bytes[18], bytes[19]]) as usize;

    let leer_f64 = |offset: usize| -> f64 {
        f64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap_or([0; 8]))
    };

    let ganancia_preamp_db = leer_f64(20);
    let graves_ganancia_db = leer_f64(28);
    let graves_frecuencia_hz = leer_f64(36);
    let graves_factor_q = leer_f64(44);
    let agudos_ganancia_db = leer_f64(52);
    let agudos_frecuencia_hz = leer_f64(60);
    let agudos_factor_q = leer_f64(68);

    let balance = leer_f64(76);
    let expansion_estereo = leer_f64(84);

    let limite_db = leer_f64(92);
    let proporcion = leer_f64(100);
    let ataque_ms = leer_f64(108);
    let lanzamiento_ms = leer_f64(116);
    let frecuencia_sidechain_hz = leer_f64(124);
    let ganancia_maquillaje_db = leer_f64(132);

    let tamano = leer_f64(140);
    let preretardo_ms = leer_f64(148);
    let amortiguacion = leer_f64(156);
    let filtro_tonal = leer_f64(164);
    let mezclado = leer_f64(172);

    let mut bandas = Vec::with_capacity(num_bandas);
    let mut cursor = 180;
    for idx in 0..num_bandas {
        if cursor + 28 > bytes.len() {
            break;
        }
        let frecuencia_hz = leer_f64(cursor);
        let factor_q = leer_f64(cursor + 8);
        let ganancia_db = leer_f64(cursor + 16);
        let tipo = match bytes[cursor + 24] {
            1 => TipoFiltro::EstanteGraves,
            2 => TipoFiltro::EstanteAgudos,
            3 => TipoFiltro::PasoBajo,
            4 => TipoFiltro::PasoAlto,
            5 => TipoFiltro::PasoBanda,
            _ => TipoFiltro::PicoParametrico,
        };
        let activo = bytes[cursor + 25] != 0;

        bandas.push(BandaParametrica {
            identificador: idx + 1,
            frecuencia_hz,
            factor_q,
            ganancia_db,
            tipo,
            canales: CanalAplicacion::EstereoAmbos,
            activo,
            color_argb: -13615201,
        });
        cursor += 28;
    }

    Some(EstadoCompletoDsp {
        nombre_perfil: String::from("Perfil APO Nativo Rust"),
        ecualizador_activo,
        modo_parametrico: true,
        frecuencia_muestreo_hz: 48000.0,
        ganancia_preamp_db,
        limitador_activo,
        bandas,
        tono: ParametrosTono {
            tono_activo,
            graves_ganancia_db,
            graves_frecuencia_hz,
            graves_factor_q,
            agudos_ganancia_db,
            agudos_frecuencia_hz,
            agudos_factor_q,
        },
        espacio: ParametrosEspacio {
            balance,
            expansion_estereo,
            modo_mono,
        },
        compresor: ParametrosCompresor {
            activo: compresor_activo,
            limite_db,
            proporcion,
            ataque_ms,
            lanzamiento_ms,
            frecuencia_sidechain_hz,
            ganancia_maquillaje_db,
        },
        reverberacion: ParametrosReverberacion {
            activo: reverb_activo,
            tamano,
            preretardo_ms,
            amortiguacion,
            filtro_tonal,
            mezclado,
        },
    })
}

// ============================================================================
// IMPLEMENTACION COM DE IAudioProcessingObject + IAudioProcessingObjectRT
// ============================================================================

#[repr(C)]
struct IAudioProcessingObjectVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    reset: unsafe extern "system" fn(*mut c_void) -> i32,
    get_latency: unsafe extern "system" fn(*mut c_void, *mut i64) -> i32,
    get_registration_properties: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
    initialize: unsafe extern "system" fn(*mut c_void, u32, *const u8) -> i32,
    is_input_format_supported:
        unsafe extern "system" fn(*mut c_void, *const c_void, *const c_void, *mut *mut c_void) -> i32,
    is_output_format_supported:
        unsafe extern "system" fn(*mut c_void, *const c_void, *const c_void, *mut *mut c_void) -> i32,
    get_input_channel_count: unsafe extern "system" fn(*mut c_void, *mut u32) -> i32,
}

#[repr(C)]
struct IAudioProcessingObjectRTVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    apo_process: unsafe extern "system" fn(
        *mut c_void,
        u32,
        *const *const ApoConnectionProperty,
        u32,
        *const *mut ApoConnectionProperty,
    ),
    calc_input_frames: unsafe extern "system" fn(*mut c_void, u32) -> u32,
    calc_output_frames: unsafe extern "system" fn(*mut c_void, u32) -> u32,
}

#[repr(C)]
struct IAudioProcessingObjectConfigVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    lock_for_process: unsafe extern "system" fn(
        *mut c_void,
        u32,
        *const c_void,
        u32,
        *const c_void,
    ) -> i32,
    unlock_for_process: unsafe extern "system" fn(*mut c_void) -> i32,
}

#[repr(C)]
struct IAudioSystemEffectsVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
}

/// Instancia COM de nuestro Objeto Procesador de Audio (APO) en Rust.
#[repr(C)]
pub struct ObjetoApoSoberano {
    vtbl_apo: *const IAudioProcessingObjectVtbl,
    vtbl_rt: *const IAudioProcessingObjectRTVtbl,
    vtbl_cfg: *const IAudioProcessingObjectConfigVtbl,
    vtbl_sfx: *const IAudioSystemEffectsVtbl,
    referencias: AtomicU32,
    cadena_dsp: CadenaMaestraDsp,
    ultima_version_ipc: u64,
    contador_bloques: u32,
    ruta_ipc: PathBuf,
    canales_bloqueados: u32,
    frecuencia_bloqueada: u32,
}

static VTBL_APO: IAudioProcessingObjectVtbl = IAudioProcessingObjectVtbl {
    query_interface: apo_query_interface,
    add_ref: apo_add_ref,
    release: apo_release,
    reset: apo_reset,
    get_latency: apo_get_latency,
    get_registration_properties: apo_get_registration_properties,
    initialize: apo_initialize,
    is_input_format_supported: apo_is_format_supported,
    is_output_format_supported: apo_is_format_supported,
    get_input_channel_count: apo_get_input_channel_count,
};

static VTBL_APO_RT: IAudioProcessingObjectRTVtbl = IAudioProcessingObjectRTVtbl {
    query_interface: apo_rt_query_interface,
    add_ref: apo_rt_add_ref,
    release: apo_rt_release,
    apo_process: apo_rt_process,
    calc_input_frames: apo_rt_calc_frames,
    calc_output_frames: apo_rt_calc_frames,
};

static VTBL_APO_CFG: IAudioProcessingObjectConfigVtbl = IAudioProcessingObjectConfigVtbl {
    query_interface: apo_cfg_query_interface,
    add_ref: apo_cfg_add_ref,
    release: apo_cfg_release,
    lock_for_process: apo_cfg_lock_for_process,
    unlock_for_process: apo_cfg_unlock_for_process,
};

static VTBL_APO_SFX: IAudioSystemEffectsVtbl = IAudioSystemEffectsVtbl {
    query_interface: apo_sfx_query_interface,
    add_ref: apo_sfx_add_ref,
    release: apo_sfx_release,
};

impl ObjetoApoSoberano {
    pub fn nuevo_en_heap() -> *mut Self {
        OBJETOS_COM_ACTIVOS.fetch_add(1, Ordering::Relaxed);
        let ruta_ipc = ruta_estado_ipc_global(None);
        let mut cadena_dsp = CadenaMaestraDsp::nueva(48000.0);
        let mut ultima_version_ipc = 0u64;
        if let Some(est) = cargar_estado_binario_ipc_si_cambio(&ruta_ipc, &mut ultima_version_ipc)
        {
            let _ = cadena_dsp.aplicar_estado(est);
        }

        let caja = Box::new(Self {
            vtbl_apo: &VTBL_APO,
            vtbl_rt: &VTBL_APO_RT,
            vtbl_cfg: &VTBL_APO_CFG,
            vtbl_sfx: &VTBL_APO_SFX,
            referencias: AtomicU32::new(1),
            cadena_dsp,
            ultima_version_ipc,
            contador_bloques: 0,
            ruta_ipc,
            canales_bloqueados: 2,
            frecuencia_bloqueada: 48000,
        });
        Box::into_raw(caja)
    }
}

#[inline]
unsafe fn obtener_base_desde_rt(este: *mut c_void) -> *mut ObjetoApoSoberano {
    let desplazamiento = std::mem::offset_of!(ObjetoApoSoberano, vtbl_rt);
    (este as *mut u8).sub(desplazamiento) as *mut ObjetoApoSoberano
}

#[inline]
unsafe fn obtener_base_desde_cfg(este: *mut c_void) -> *mut ObjetoApoSoberano {
    let desplazamiento = std::mem::offset_of!(ObjetoApoSoberano, vtbl_cfg);
    (este as *mut u8).sub(desplazamiento) as *mut ObjetoApoSoberano
}

#[inline]
unsafe fn obtener_base_desde_sfx(este: *mut c_void) -> *mut ObjetoApoSoberano {
    let desplazamiento = std::mem::offset_of!(ObjetoApoSoberano, vtbl_sfx);
    (este as *mut u8).sub(desplazamiento) as *mut ObjetoApoSoberano
}

unsafe extern "system" fn apo_query_interface(
    este: *mut c_void,
    riid: *const Guid,
    ppv: *mut *mut c_void,
) -> i32 {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let obj = este as *mut ObjetoApoSoberano;
    let iid = *riid;

    if iid == IID_IUNKNOWN || iid == IID_IAUDIO_PROCESSING_OBJECT {
        *ppv = obj as *mut c_void;
        apo_add_ref(este);
        S_OK
    } else if iid == IID_IAUDIO_PROCESSING_OBJECT_RT {
        *ppv = ptr::addr_of_mut!((*obj).vtbl_rt) as *mut c_void;
        apo_add_ref(este);
        S_OK
    } else if iid == IID_IAUDIO_PROCESSING_OBJECT_CONFIG {
        *ppv = ptr::addr_of_mut!((*obj).vtbl_cfg) as *mut c_void;
        apo_add_ref(este);
        S_OK
    } else if iid == IID_IAUDIO_SYSTEM_EFFECTS {
        *ppv = ptr::addr_of_mut!((*obj).vtbl_sfx) as *mut c_void;
        apo_add_ref(este);
        S_OK
    } else {
        *ppv = ptr::null_mut();
        E_NOINTERFACE
    }
}

unsafe extern "system" fn apo_add_ref(este: *mut c_void) -> u32 {
    let obj = &*(este as *const ObjetoApoSoberano);
    obj.referencias.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn apo_release(este: *mut c_void) -> u32 {
    let obj = este as *mut ObjetoApoSoberano;
    let prev = (*obj).referencias.fetch_sub(1, Ordering::Release);
    if prev == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        let _ = Box::from_raw(obj);
        OBJETOS_COM_ACTIVOS.fetch_sub(1, Ordering::Relaxed);
        0
    } else {
        prev - 1
    }
}

unsafe extern "system" fn apo_reset(este: *mut c_void) -> i32 {
    let obj = &mut *(este as *mut ObjetoApoSoberano);
    let estado_actual = obj.cadena_dsp.obtener_estado().clone();
    let _ = obj.cadena_dsp.aplicar_estado(estado_actual);
    S_OK
}

unsafe extern "system" fn apo_get_latency(_este: *mut c_void, p_time: *mut i64) -> i32 {
    if p_time.is_null() {
        return E_POINTER;
    }
    *p_time = 0; // Procesamiento Biquad IIR con 0 ms de latencia de bufer
    S_OK
}

unsafe extern "system" fn apo_get_registration_properties(
    _este: *mut c_void,
    pp_reg_props: *mut *mut c_void,
) -> i32 {
    if pp_reg_props.is_null() {
        return E_POINTER;
    }
    let tamano = std::mem::size_of::<PropiedadesRegistroApo>();
    let bloque = CoTaskMemAlloc(tamano) as *mut PropiedadesRegistroApo;
    if bloque.is_null() {
        *pp_reg_props = ptr::null_mut();
        return E_OUTOFMEMORY;
    }
    ptr::write_bytes(bloque as *mut u8, 0, tamano);
    let props = &mut *bloque;
    props.clsid = CLSID_APO_SOBERANO_RUST;
    props.banderas = BANDERAS_APO_REGISTRO;
    copiar_texto_utf16(&mut props.nombre_amigable, NOMBRE_AMIGABLE_APO);
    copiar_texto_utf16(&mut props.derechos, DERECHOS_APO);
    props.version_mayor = 2;
    props.version_menor = 0;
    props.min_conexiones_entrada = 1;
    props.max_conexiones_entrada = 1;
    props.min_conexiones_salida = 1;
    props.max_conexiones_salida = 1;
    props.max_instancias = u32::MAX;
    props.numero_interfaces = 1;
    props.lista_interfaces = [IID_IAUDIO_PROCESSING_OBJECT];
    *pp_reg_props = bloque as *mut c_void;
    S_OK
}

unsafe extern "system" fn apo_initialize(
    _este: *mut c_void,
    _cb_data_size: u32,
    _pby_data: *const u8,
) -> i32 {
    S_OK
}

unsafe extern "system" fn apo_is_format_supported(
    _este: *mut c_void,
    _p_opposite_format: *const c_void,
    p_requested_format: *const c_void,
    pp_supported_format: *mut *mut c_void,
) -> i32 {
    if pp_supported_format.is_null() || p_requested_format.is_null() {
        return E_POINTER;
    }
    let tipo_medio = p_requested_format as *mut c_void;
    match leer_formato_tipo_medio(tipo_medio) {
        Some((canales, _, true)) if canales >= 1 => {
            // El llamador libera el puntero devuelto: incrementar su conteo de referencias.
            let vtbl = *(tipo_medio as *const *const IAudioMediaTypeVtbl);
            ((*vtbl).add_ref)(tipo_medio);
            *pp_supported_format = tipo_medio;
            S_OK
        }
        _ => {
            *pp_supported_format = ptr::null_mut();
            APOERR_FORMAT_NOT_SUPPORTED
        }
    }
}

unsafe extern "system" fn apo_get_input_channel_count(
    _este: *mut c_void,
    pu32_channel_count: *mut u32,
) -> i32 {
    if pu32_channel_count.is_null() {
        return E_POINTER;
    }
    *pu32_channel_count = 2;
    S_OK
}

// Delegadores para IAudioProcessingObjectRT
unsafe extern "system" fn apo_rt_query_interface(
    este: *mut c_void,
    riid: *const Guid,
    ppv: *mut *mut c_void,
) -> i32 {
    apo_query_interface(obtener_base_desde_rt(este) as *mut c_void, riid, ppv)
}

unsafe extern "system" fn apo_rt_add_ref(este: *mut c_void) -> u32 {
    apo_add_ref(obtener_base_desde_rt(este) as *mut c_void)
}

unsafe extern "system" fn apo_rt_release(este: *mut c_void) -> u32 {
    apo_release(obtener_base_desde_rt(este) as *mut c_void)
}

/// Metodo critico en tiempo real invocado por `audiodg.exe` sobre cada bloque de muestras de salida.
unsafe extern "system" fn apo_rt_process(
    este: *mut c_void,
    _u32_num_input_connections: u32,
    pp_input_connections: *const *const ApoConnectionProperty,
    _u32_num_output_connections: u32,
    pp_output_connections: *const *mut ApoConnectionProperty,
) {
    if pp_input_connections.is_null() || pp_output_connections.is_null() {
        return;
    }
    let conn_entrada = *pp_input_connections;
    let conn_salida = *pp_output_connections;
    if conn_entrada.is_null() || conn_salida.is_null() {
        return;
    }

    let obj = &mut *obtener_base_desde_rt(este);

    // Cada 16 bloques (~160 ms) verificar si el usuario actualizo los parametros en la interfaz
    obj.contador_bloques = obj.contador_bloques.wrapping_add(1);
    if (obj.contador_bloques & 15) == 0 {
        if let Some(mut nuevo_estado) =
            cargar_estado_binario_ipc_si_cambio(&obj.ruta_ipc, &mut obj.ultima_version_ipc)
        {
            nuevo_estado.frecuencia_muestreo_hz = obj.frecuencia_bloqueada as f64;
            let _ = obj.cadena_dsp.aplicar_estado(nuevo_estado);
        }
    }

    let fotogramas = (*conn_entrada).u32_valid_frame_count as usize;
    let banderas = (*conn_entrada).u32_buffer_flags;

    if banderas == BUFFER_SILENT || (*conn_entrada).p_buffer == 0 || (*conn_salida).p_buffer == 0 {
        (*conn_salida).u32_valid_frame_count = (*conn_entrada).u32_valid_frame_count;
        (*conn_salida).u32_buffer_flags = BUFFER_SILENT;
        return;
    }

    let ptr_in = (*conn_entrada).p_buffer as *const f32;
    let ptr_out = (*conn_salida).p_buffer as *mut f32;
    let canales = obj.canales_bloqueados.max(1) as usize;

    for i in 0..fotogramas {
        let base = i * canales;
        let m_izq = *ptr_in.add(base);
        let m_der = if canales > 1 { *ptr_in.add(base + 1) } else { m_izq };
        let (s_izq, s_der) = obj.cadena_dsp.procesar_par_estereo(m_izq, m_der);
        *ptr_out.add(base) = s_izq;
        if canales > 1 {
            *ptr_out.add(base + 1) = s_der;
        }
        // Canales envolventes (centro, LFE, traseros) se copian sin modificar
        if ptr_in != ptr_out as *const f32 {
            for c in 2..canales {
                *ptr_out.add(base + c) = *ptr_in.add(base + c);
            }
        }
    }

    (*conn_salida).u32_valid_frame_count = (*conn_entrada).u32_valid_frame_count;
    (*conn_salida).u32_buffer_flags = BUFFER_VALID;
}

unsafe extern "system" fn apo_rt_calc_frames(_este: *mut c_void, u32_frames: u32) -> u32 {
    u32_frames
}

// Delegadores para IAudioProcessingObjectConfiguration
unsafe extern "system" fn apo_cfg_query_interface(
    este: *mut c_void,
    riid: *const Guid,
    ppv: *mut *mut c_void,
) -> i32 {
    apo_query_interface(obtener_base_desde_cfg(este) as *mut c_void, riid, ppv)
}

unsafe extern "system" fn apo_cfg_add_ref(este: *mut c_void) -> u32 {
    apo_add_ref(obtener_base_desde_cfg(este) as *mut c_void)
}

unsafe extern "system" fn apo_cfg_release(este: *mut c_void) -> u32 {
    apo_release(obtener_base_desde_cfg(este) as *mut c_void)
}

unsafe extern "system" fn apo_cfg_lock_for_process(
    este: *mut c_void,
    num_in: u32,
    pp_in: *const c_void,
    _num_out: u32,
    _pp_out: *const c_void,
) -> i32 {
    if num_in == 0 || pp_in.is_null() {
        return E_POINTER;
    }
    let obj = &mut *obtener_base_desde_cfg(este);
    let descriptores = pp_in as *const *const DescriptorConexionApo;
    let descriptor = *descriptores;
    if descriptor.is_null() {
        return E_POINTER;
    }
    let Some((canales, frecuencia, es_float)) = leer_formato_tipo_medio((*descriptor).p_formato)
    else {
        return APOERR_FORMAT_NOT_SUPPORTED;
    };
    if !es_float {
        return APOERR_FORMAT_NOT_SUPPORTED;
    }
    obj.canales_bloqueados = canales as u32;
    obj.frecuencia_bloqueada = frecuencia;

    let mut estado = obj.cadena_dsp.obtener_estado().clone();
    estado.frecuencia_muestreo_hz = frecuencia as f64;
    let _ = obj.cadena_dsp.aplicar_estado(estado);
    S_OK
}

unsafe extern "system" fn apo_cfg_unlock_for_process(_este: *mut c_void) -> i32 {
    S_OK
}

// Delegadores para IAudioSystemEffects
unsafe extern "system" fn apo_sfx_query_interface(
    este: *mut c_void,
    riid: *const Guid,
    ppv: *mut *mut c_void,
) -> i32 {
    apo_query_interface(obtener_base_desde_sfx(este) as *mut c_void, riid, ppv)
}

unsafe extern "system" fn apo_sfx_add_ref(este: *mut c_void) -> u32 {
    apo_add_ref(obtener_base_desde_sfx(este) as *mut c_void)
}

unsafe extern "system" fn apo_sfx_release(este: *mut c_void) -> u32 {
    apo_release(obtener_base_desde_sfx(este) as *mut c_void)
}

// ============================================================================
// IClassFactory Y PUNTOS DE ENTRADA COM DLL
// ============================================================================

#[allow(dead_code)]
#[repr(C)]
struct IClassFactoryVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    create_instance:
        unsafe extern "system" fn(*mut c_void, *mut c_void, *const Guid, *mut *mut c_void) -> i32,
    lock_server: unsafe extern "system" fn(*mut c_void, i32) -> i32,
}

#[allow(dead_code)]
#[repr(C)]
struct FabricaApoCom {
    vtbl: *const IClassFactoryVtbl,
}

unsafe impl Sync for FabricaApoCom {}

#[allow(dead_code)]
static VTBL_FABRICA: IClassFactoryVtbl = IClassFactoryVtbl {
    query_interface: fabrica_query_interface,
    add_ref: fabrica_add_ref,
    release: fabrica_release,
    create_instance: fabrica_create_instance,
    lock_server: fabrica_lock_server,
};

#[allow(dead_code)]
static INSTANCIA_FABRICA: FabricaApoCom = FabricaApoCom {
    vtbl: &VTBL_FABRICA,
};

#[allow(dead_code)]
unsafe extern "system" fn fabrica_query_interface(
    este: *mut c_void,
    riid: *const Guid,
    ppv: *mut *mut c_void,
) -> i32 {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let iid = *riid;
    if iid == IID_IUNKNOWN || iid == IID_ICLASS_FACTORY {
        *ppv = este;
        S_OK
    } else {
        *ppv = ptr::null_mut();
        E_NOINTERFACE
    }
}

#[allow(dead_code)]
unsafe extern "system" fn fabrica_add_ref(_este: *mut c_void) -> u32 {
    1
}

#[allow(dead_code)]
unsafe extern "system" fn fabrica_release(_este: *mut c_void) -> u32 {
    1
}

#[allow(dead_code)]
unsafe extern "system" fn fabrica_create_instance(
    _este: *mut c_void,
    p_unk_outer: *mut c_void,
    riid: *const Guid,
    ppv: *mut *mut c_void,
) -> i32 {
    if ppv.is_null() {
        return E_POINTER;
    }
    *ppv = ptr::null_mut();
    if !p_unk_outer.is_null() {
        return CLASS_E_NOAGGREGATION;
    }
    let instancia = ObjetoApoSoberano::nuevo_en_heap();
    let hr = apo_query_interface(instancia as *mut c_void, riid, ppv);
    apo_release(instancia as *mut c_void);
    hr
}

#[allow(dead_code)]
unsafe extern "system" fn fabrica_lock_server(_este: *mut c_void, f_lock: i32) -> i32 {
    if f_lock != 0 {
        OBJETOS_COM_ACTIVOS.fetch_add(1, Ordering::Relaxed);
    } else {
        OBJETOS_COM_ACTIVOS.fetch_sub(1, Ordering::Relaxed);
    }
    S_OK
}

#[cfg(feature = "dll_com")]
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const Guid,
    riid: *const Guid,
    ppv: *mut *mut c_void,
) -> i32 {
    if ppv.is_null() || rclsid.is_null() || riid.is_null() {
        return E_POINTER;
    }
    if *rclsid != CLSID_APO_SOBERANO_RUST {
        *ppv = ptr::null_mut();
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    fabrica_query_interface(
        (&INSTANCIA_FABRICA as *const FabricaApoCom) as *mut c_void,
        riid,
        ppv,
    )
}

#[cfg(feature = "dll_com")]
#[no_mangle]
pub unsafe extern "system" fn DllCanUnloadNow() -> i32 {
    if OBJETOS_COM_ACTIVOS.load(Ordering::Relaxed) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

/// El registro COM y del motor de audio lo realiza `instalador_apo.rs` (con elevacion UAC),
/// ya que necesita conocer la ruta final de la DLL en `Program Files`.
#[cfg(feature = "dll_com")]
#[no_mangle]
pub unsafe extern "system" fn DllRegisterServer() -> i32 {
    S_OK
}

#[cfg(feature = "dll_com")]
#[no_mangle]
pub unsafe extern "system" fn DllUnregisterServer() -> i32 {
    S_OK
}
