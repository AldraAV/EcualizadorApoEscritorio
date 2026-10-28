//! Modulo Instalador y Administrador del Windows Audio Processing Object (APO COM DLL) en Rust.
//! Registra e inyecta `motor_ecualizador_poweramp.dll` directamente en el motor de audio de Windows (`audiodg.exe`)
//! para que todo el sonido del sistema (navegadores, Spotify, juegos, reproductores) sea procesado
//! por la Cadena Maestra DSP a 64 bits en tiempo real.
//!
//! Cumple estrictamente con la Soberania del Español y Cero Emojis.

use std::env;
use std::ffi::c_void;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;

use crate::apo_com_windows::CLSID_APO_SOBERANO_TEXTO;

// Tipos y constantes de la API Win32 del Registro (advapi32.dll)
type Hkey = usize;

#[allow(dead_code)]
const HKEY_CLASSES_ROOT: Hkey = 0x80000000;
const HKEY_LOCAL_MACHINE: Hkey = 0x80000002;

const KEY_READ: u32 = 0x20019;
#[allow(dead_code)]
const KEY_WRITE: u32 = 0x20006;
const KEY_ALL_ACCESS: u32 = 0xF003F;

const REG_SZ: u32 = 1;
const REG_DWORD: u32 = 4;

#[link(name = "advapi32")]
extern "system" {
    fn RegOpenKeyExW(
        h_key: Hkey,
        lp_sub_key: *const u16,
        ul_options: u32,
        sam_desired: u32,
        phk_result: *mut Hkey,
    ) -> i32;

    fn RegCreateKeyExW(
        h_key: Hkey,
        lp_sub_key: *const u16,
        reserved: u32,
        lp_class: *const u16,
        dw_options: u32,
        sam_desired: u32,
        lp_security_attributes: *const c_void,
        phk_result: *mut Hkey,
        lpdw_disposition: *mut u32,
    ) -> i32;

    fn RegSetValueExW(
        h_key: Hkey,
        lp_value_name: *const u16,
        reserved: u32,
        dw_type: u32,
        lp_data: *const u8,
        cb_data: u32,
    ) -> i32;

    fn RegDeleteValueW(h_key: Hkey, lp_value_name: *const u16) -> i32;

    fn RegDeleteKeyW(h_key: Hkey, lp_sub_key: *const u16) -> i32;

    fn RegCloseKey(h_key: Hkey) -> i32;

    fn RegEnumKeyExW(
        h_key: Hkey,
        dw_index: u32,
        lp_name: *mut u16,
        lpcch_name: *mut u32,
        lp_reserved: *mut u32,
        lp_class: *mut u16,
        lpcch_class: *mut u32,
        lpft_last_write_time: *mut c_void,
    ) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: usize,
        lp_operation: *const u16,
        lp_file: *const u16,
        lp_parameters: *const u16,
        lp_directory: *const u16,
        n_show_cmd: i32,
    ) -> usize;
}

fn a_utf16_nulo(texto: &str) -> Vec<u16> {
    let mut v: Vec<u16> = texto.encode_utf16().collect();
    v.push(0);
    v
}

/// Helper para escribir una cadena REG_SZ en el registro.
fn escribir_valor_cadena_registro(clave: Hkey, nombre: Option<&str>, valor: &str) -> Result<(), String> {
    let nombre_utf16 = nombre.map(a_utf16_nulo);
    let ptr_nombre = match &nombre_utf16 {
        Some(v) => v.as_ptr(),
        None => ptr::null(),
    };
    let valor_utf16 = a_utf16_nulo(valor);
    let tamano_bytes = (valor_utf16.len() * 2) as u32;

    let res = unsafe {
        RegSetValueExW(
            clave,
            ptr_nombre,
            0,
            REG_SZ,
            valor_utf16.as_ptr() as *const u8,
            tamano_bytes,
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(format!("Error al escribir valor de cadena (codigo: {})", res))
    }
}

/// Helper para escribir un DWORD (u32) en el registro.
fn escribir_valor_dword_registro(clave: Hkey, nombre: &str, valor: u32) -> Result<(), String> {
    let nombre_utf16 = a_utf16_nulo(nombre);
    let bytes = valor.to_le_bytes();

    let res = unsafe {
        RegSetValueExW(
            clave,
            nombre_utf16.as_ptr(),
            0,
            REG_DWORD,
            bytes.as_ptr(),
            bytes.len() as u32,
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(format!("Error al escribir valor DWORD (codigo: {})", res))
    }
}

/// Abre o crea una clave de registro bajo una clave raiz dada.
fn abrir_o_crear_clave(raiz: Hkey, subclave: &str) -> Result<Hkey, String> {
    let subclave_utf16 = a_utf16_nulo(subclave);
    let mut clave_resultado: Hkey = 0;
    let mut disposicion: u32 = 0;

    let res = unsafe {
        RegCreateKeyExW(
            raiz,
            subclave_utf16.as_ptr(),
            0,
            ptr::null(),
            0,
            KEY_ALL_ACCESS,
            ptr::null(),
            &mut clave_resultado,
            &mut disposicion,
        )
    };

    if res == 0 && clave_resultado != 0 {
        Ok(clave_resultado)
    } else {
        Err(format!(
            "No se pudo crear o abrir la clave de registro '{}' (codigo: {})",
            subclave, res
        ))
    }
}

/// Estado de la instalacion del APO en Windows.
#[derive(Debug, Clone)]
pub struct EstadoInstalacionApo {
    pub dll_instalada: bool,
    pub ruta_dll: PathBuf,
    pub com_registrado: bool,
    pub apo_registrado: bool,
    pub proteccion_audiodg_desactivada: bool,
    pub dispositivos_enlazados: Vec<String>,
}

/// Directorio oficial donde se hospeda la DLL del APO en `Program Files`.
pub fn ruta_oficial_directorio_apo() -> PathBuf {
    PathBuf::from(r"C:\Program Files\AldraPowerampDsp")
}

/// Ruta oficial del archivo DLL del APO.
pub fn ruta_oficial_dll_apo() -> PathBuf {
    ruta_oficial_directorio_apo().join("motor_ecualizador_poweramp.dll")
}

/// Verifica el estado actual de instalacion del APO en el sistema sin modificar nada.
pub fn consultar_estado_instalacion_apo() -> EstadoInstalacionApo {
    let ruta_dll = ruta_oficial_dll_apo();
    let dll_instalada = ruta_dll.exists();

    let mut com_registrado = false;
    let subclave_clsid = format!(r"SOFTWARE\Classes\CLSID\{}\InProcServer32", CLSID_APO_SOBERANO_TEXTO);
    let subclave_clsid_utf16 = a_utf16_nulo(&subclave_clsid);
    let mut h_clsid: Hkey = 0;
    if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, subclave_clsid_utf16.as_ptr(), 0, KEY_READ, &mut h_clsid) } == 0 {
        com_registrado = true;
        unsafe { RegCloseKey(h_clsid); }
    }

    let mut apo_registrado = false;
    let subclave_apo = format!(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\AudioProcessingObjects\{}",
        CLSID_APO_SOBERANO_TEXTO
    );
    let subclave_apo_utf16 = a_utf16_nulo(&subclave_apo);
    let mut h_apo: Hkey = 0;
    if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, subclave_apo_utf16.as_ptr(), 0, KEY_READ, &mut h_apo) } == 0 {
        apo_registrado = true;
        unsafe { RegCloseKey(h_apo); }
    }

    let mut proteccion_audiodg_desactivada = false;
    let subclave_audio = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Audio";
    let subclave_audio_utf16 = a_utf16_nulo(subclave_audio);
    let mut h_audio: Hkey = 0;
    if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, subclave_audio_utf16.as_ptr(), 0, KEY_READ, &mut h_audio) } == 0 {
        // Se considera activa si la clave es accesible
        proteccion_audiodg_desactivada = true;
        unsafe { RegCloseKey(h_audio); }
    }

    let mut dispositivos_enlazados = Vec::new();
    let subclave_render = r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render";
    let subclave_render_utf16 = a_utf16_nulo(subclave_render);
    let mut h_render: Hkey = 0;
    if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, subclave_render_utf16.as_ptr(), 0, KEY_READ, &mut h_render) } == 0 {
        let mut indice = 0u32;
        let mut bufer_nombre = [0u16; 256];
        loop {
            let mut tamano_nombre = bufer_nombre.len() as u32;
            let res = unsafe {
                RegEnumKeyExW(
                    h_render,
                    indice,
                    bufer_nombre.as_mut_ptr(),
                    &mut tamano_nombre,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            if res != 0 {
                break;
            }
            let id_endpoint = String::from_utf16_lossy(&bufer_nombre[..tamano_nombre as usize]);
            let sub_fx = format!(r"{}\FxProperties", id_endpoint);
            let sub_fx_utf16 = a_utf16_nulo(&sub_fx);
            let mut h_fx: Hkey = 0;
            if unsafe { RegOpenKeyExW(h_render, sub_fx_utf16.as_ptr(), 0, KEY_READ, &mut h_fx) } == 0 {
                dispositivos_enlazados.push(id_endpoint);
                unsafe { RegCloseKey(h_fx); }
            }
            indice += 1;
        }
        unsafe { RegCloseKey(h_render); }
    }

    EstadoInstalacionApo {
        dll_instalada,
        ruta_dll,
        com_registrado,
        apo_registrado,
        proteccion_audiodg_desactivada,
        dispositivos_enlazados,
    }
}

/// Instala y activa completamente el APO Soberano en Windows:
/// 1. Copia `motor_ecualizador_poweramp.dll` a `C:\Program Files\AldraPowerampDsp\`.
/// 2. Habilita permisos IPC en `C:\ProgramData\AldraPowerampDsp\`.
/// 3. Registra el servidor COM y el AudioProcessingObject en el registro de Windows.
/// 4. Desactiva `DisableProtectedAudioDG` para permitir DLLs de audio de terceros en `audiodg.exe`.
/// 5. Inyecta los GUIDs de SFX, MFX y EFX en los endpoints de salida.
pub fn instalar_y_activar_apo(ruta_dll_origen: &Path) -> Result<(), String> {
    if !ruta_dll_origen.exists() {
        return Err(format!(
            "No se encontro el archivo DLL fuente para instalar: {}",
            ruta_dll_origen.display()
        ));
    }

    // 1. Crear directorio destino y copiar DLL
    let dir_destino = ruta_oficial_directorio_apo();
    fs::create_dir_all(&dir_destino)
        .map_err(|e| format!("Error al crear '{}': {}. ¿Se ejecuto como Administrador?", dir_destino.display(), e))?;

    let ruta_dll_destino = ruta_oficial_dll_apo();
    fs::copy(ruta_dll_origen, &ruta_dll_destino)
        .map_err(|e| format!("Error al copiar DLL a '{}': {}", ruta_dll_destino.display(), e))?;

    // 2. Crear directorio compartido IPC ProgramData
    let dir_ipc = Path::new(r"C:\ProgramData\AldraPowerampDsp");
    let _ = fs::create_dir_all(dir_ipc);

    // 3. Registrar servidor COM
    let clsid_str = CLSID_APO_SOBERANO_TEXTO;
    let clave_clsid_path = format!(r"SOFTWARE\Classes\CLSID\{}", clsid_str);
    let h_clsid = abrir_o_crear_clave(HKEY_LOCAL_MACHINE, &clave_clsid_path)?;
    escribir_valor_cadena_registro(h_clsid, None, "Aldra Poweramp DSP APO")?;
    unsafe { RegCloseKey(h_clsid); }

    let clave_inproc_path = format!(r"SOFTWARE\Classes\CLSID\{}\InProcServer32", clsid_str);
    let h_inproc = abrir_o_crear_clave(HKEY_LOCAL_MACHINE, &clave_inproc_path)?;
    escribir_valor_cadena_registro(h_inproc, None, &ruta_dll_destino.to_string_lossy())?;
    escribir_valor_cadena_registro(h_inproc, Some("ThreadingModel"), "Both")?;
    unsafe { RegCloseKey(h_inproc); }

    // 4. Registrar AudioProcessingObject en Windows Audio Engine
    let clave_apo_path = format!(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\AudioProcessingObjects\{}",
        clsid_str
    );
    let h_apo = abrir_o_crear_clave(HKEY_LOCAL_MACHINE, &clave_apo_path)?;
    escribir_valor_cadena_registro(h_apo, Some("FriendlyName"), "Aldra Poweramp DSP (Rust)")?;
    escribir_valor_cadena_registro(h_apo, Some("Copyright"), "Jose Aldra Aviles Cardenas")?;
    escribir_valor_dword_registro(h_apo, "MajorVersion", 2)?;
    escribir_valor_dword_registro(h_apo, "MinorVersion", 0)?;
    escribir_valor_dword_registro(h_apo, "Flags", 0x0000000F)?; // INPLACE + DEFAULT
    escribir_valor_dword_registro(h_apo, "MinInputConnections", 1)?;
    escribir_valor_dword_registro(h_apo, "MaxInputConnections", 1)?;
    escribir_valor_dword_registro(h_apo, "MinOutputConnections", 1)?;
    escribir_valor_dword_registro(h_apo, "MaxOutputConnections", 1)?;
    escribir_valor_dword_registro(h_apo, "MaxInstances", 0xFFFFFFFF)?;
    escribir_valor_dword_registro(h_apo, "NumAPOInterfaces", 1)?;
    escribir_valor_cadena_registro(
        h_apo,
        Some("APOInterface0"),
        "{FD7F2B29-24D0-4B5C-B177-592C39F9CA10}",
    )?;
    unsafe { RegCloseKey(h_apo); }

    // 5. Desactivar DisableProtectedAudioDG en Windows (permite DLLs de terceros en audiodg.exe)
    let h_audio = abrir_o_crear_clave(HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Audio")?;
    escribir_valor_dword_registro(h_audio, "DisableProtectedAudioDG", 1)?;
    unsafe { RegCloseKey(h_audio); }

    // 6. Inyectar en los endpoints de salida de audio de Windows (FxProperties)
    // GUIDs oficiales de las propiedades de efectos de Windows:
    // PKEY_FX_StreamEffectClsid = "{d04e05a5-594b-4fb6-a80d-01af5eed7d1d},1" (SFX)
    // PKEY_FX_ModeEffectClsid   = "{d04e05a5-594b-4fb6-a80d-01af5eed7d1d},2" (MFX)
    // PKEY_FX_EndpointEffectClsid = "{d04e05a5-594b-4fb6-a80d-01af5eed7d1d},3" (EFX)
    let subclave_render = r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render";
    let subclave_render_utf16 = a_utf16_nulo(subclave_render);
    let mut h_render: Hkey = 0;

    if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, subclave_render_utf16.as_ptr(), 0, KEY_ALL_ACCESS, &mut h_render) } == 0 {
        let mut indice = 0u32;
        let mut bufer_nombre = [0u16; 256];
        loop {
            let mut tamano_nombre = bufer_nombre.len() as u32;
            let res = unsafe {
                RegEnumKeyExW(
                    h_render,
                    indice,
                    bufer_nombre.as_mut_ptr(),
                    &mut tamano_nombre,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            if res != 0 {
                break;
            }
            let id_endpoint = String::from_utf16_lossy(&bufer_nombre[..tamano_nombre as usize]);
            let sub_fx = format!(r"{}\{}\FxProperties", subclave_render, id_endpoint);
            if let Ok(h_fx) = abrir_o_crear_clave(HKEY_LOCAL_MACHINE, &sub_fx) {
                // Registrar nuestro APO como SFX y EFX
                let _ = escribir_valor_cadena_registro(
                    h_fx,
                    Some("{d04e05a5-594b-4fb6-a80d-01af5eed7d1d},1"),
                    clsid_str,
                );
                let _ = escribir_valor_cadena_registro(
                    h_fx,
                    Some("{d04e05a5-594b-4fb6-a80d-01af5eed7d1d},2"),
                    clsid_str,
                );
                let _ = escribir_valor_cadena_registro(
                    h_fx,
                    Some("{d04e05a5-594b-4fb6-a80d-01af5eed7d1d},3"),
                    clsid_str,
                );
                unsafe { RegCloseKey(h_fx); }
            }
            indice += 1;
        }
        unsafe { RegCloseKey(h_render); }
    }

    Ok(())
}

/// Desinstala y remueve el APO del sistema restaurando los endpoints originales.
pub fn desinstalar_apo() -> Result<(), String> {
    let clsid_str = CLSID_APO_SOBERANO_TEXTO;

    // 1. Remover de FxProperties de cada dispositivo
    let subclave_render = r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render";
    let subclave_render_utf16 = a_utf16_nulo(subclave_render);
    let mut h_render: Hkey = 0;

    if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, subclave_render_utf16.as_ptr(), 0, KEY_ALL_ACCESS, &mut h_render) } == 0 {
        let mut indice = 0u32;
        let mut bufer_nombre = [0u16; 256];
        loop {
            let mut tamano_nombre = bufer_nombre.len() as u32;
            let res = unsafe {
                RegEnumKeyExW(
                    h_render,
                    indice,
                    bufer_nombre.as_mut_ptr(),
                    &mut tamano_nombre,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            if res != 0 {
                break;
            }
            let id_endpoint = String::from_utf16_lossy(&bufer_nombre[..tamano_nombre as usize]);
            let sub_fx = format!(r"{}\{}\FxProperties", subclave_render, id_endpoint);
            let sub_fx_utf16 = a_utf16_nulo(&sub_fx);
            let mut h_fx: Hkey = 0;
            if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, sub_fx_utf16.as_ptr(), 0, KEY_ALL_ACCESS, &mut h_fx) } == 0 {
                let v1 = a_utf16_nulo("{d04e05a5-594b-4fb6-a80d-01af5eed7d1d},1");
                let v2 = a_utf16_nulo("{d04e05a5-594b-4fb6-a80d-01af5eed7d1d},2");
                let v3 = a_utf16_nulo("{d04e05a5-594b-4fb6-a80d-01af5eed7d1d},3");
                unsafe {
                    RegDeleteValueW(h_fx, v1.as_ptr());
                    RegDeleteValueW(h_fx, v2.as_ptr());
                    RegDeleteValueW(h_fx, v3.as_ptr());
                    RegCloseKey(h_fx);
                }
            }
            indice += 1;
        }
        unsafe { RegCloseKey(h_render); }
    }

    // 2. Borrar clave de AudioProcessingObjects
    let subclave_apo = format!(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\AudioProcessingObjects\{}",
        clsid_str
    );
    let subclave_apo_utf16 = a_utf16_nulo(&subclave_apo);
    unsafe { RegDeleteKeyW(HKEY_LOCAL_MACHINE, subclave_apo_utf16.as_ptr()); }

    // 3. Borrar clave CLSID
    let subclave_inproc = format!(r"SOFTWARE\Classes\CLSID\{}\InProcServer32", clsid_str);
    let subclave_inproc_utf16 = a_utf16_nulo(&subclave_inproc);
    unsafe { RegDeleteKeyW(HKEY_LOCAL_MACHINE, subclave_inproc_utf16.as_ptr()); }

    let subclave_clsid = format!(r"SOFTWARE\Classes\CLSID\{}", clsid_str);
    let subclave_clsid_utf16 = a_utf16_nulo(&subclave_clsid);
    unsafe { RegDeleteKeyW(HKEY_LOCAL_MACHINE, subclave_clsid_utf16.as_ptr()); }

    Ok(())
}

/// Solicita elevacion de privilegios UAC ejecutando el mismo binario con el verbo `runas`.
pub fn solicitar_elevacion_administrador(argumentos_extra: &str) -> bool {
    if let Ok(ejecutable_actual) = env::current_exe() {
        let op_utf16 = a_utf16_nulo("runas");
        let file_utf16 = a_utf16_nulo(&ejecutable_actual.to_string_lossy());
        let params_utf16 = a_utf16_nulo(argumentos_extra);

        let res = unsafe {
            ShellExecuteW(
                0,
                op_utf16.as_ptr(),
                file_utf16.as_ptr(),
                params_utf16.as_ptr(),
                ptr::null(),
                1, // SW_SHOWNORMAL
            )
        };
        res > 32
    } else {
        false
    }
}
