import re
from pathlib import Path

# 1. Parchear Cargo.toml
cargo_path = Path("Cargo.toml")
if cargo_path.exists():
    contenido = cargo_path.read_text(encoding="utf-8")
    contenido = re.sub(
        r'iced\s*=\s*\{[^}]*features\s*=\s*\[([^\]]*)\][^}]*\}',
        'iced = { version = "0.14.0", default-features = false, features = ["wgpu", "canvas", "tokio", "x11", "wayland"] }',
        contenido
    )
    cargo_path.write_text(contenido, encoding="utf-8")
    print("[+] Cargo.toml actualizado con features x11 y wayland.")

# 2. Parchear captura_wasapi_loopback.rs
wasapi_path = Path("src/captura_wasapi_loopback.rs")
if wasapi_path.exists():
    texto = wasapi_path.read_text(encoding="utf-8")
    if "mod windows_impl" not in texto:
        ancla = "const CLSID_MMDEVICE_ENUMERATOR: Guid = Guid {"
        stubs_linux = """#[cfg(not(target_os = "windows"))]
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

    const CLSID_MMDEVICE_ENUMERATOR: Guid = Guid {"""
        texto = texto.replace(ancla, stubs_linux, 1)
        texto += "\n}\n#[cfg(target_os = \"windows\")]\npub use windows_impl::*;\n"
        wasapi_path.write_text(texto, encoding="utf-8")
        print("[+] captura_wasapi_loopback.rs acondicionado para Linux.")

# 3. Parchear instalador_apo.rs
apo_path = Path("src/instalador_apo.rs")
if apo_path.exists():
    texto = apo_path.read_text(encoding="utf-8")
    if "mod windows_impl" not in texto:
        ancla = "type Hkey = usize;"
        stubs_linux = """#[derive(Debug, Clone, Default)]
pub struct EstadoInstalacionApo {
    pub dll_instalada: bool,
    pub ruta_dll: PathBuf,
    pub com_registrado: bool,
    pub apo_registrado: bool,
    pub proteccion_audiodg_desactivada: bool,
    pub dispositivos_enlazados: Vec<String>,
}

#[cfg(not(target_os = "windows"))]
pub fn ruta_oficial_directorio_apo() -> PathBuf {
    PathBuf::from("/etc/aldra_poweramp")
}

#[cfg(not(target_os = "windows"))]
pub fn ruta_oficial_dll_apo() -> PathBuf {
    PathBuf::from("/etc/aldra_poweramp/motor_ecualizador_poweramp.so")
}

#[cfg(not(target_os = "windows"))]
pub fn consultar_estado_instalacion_apo() -> EstadoInstalacionApo {
    EstadoInstalacionApo::default()
}

#[cfg(not(target_os = "windows"))]
pub fn instalar_y_activar_apo(_ruta: &Path) -> Result<(), String> {
    Err(String::from("El subsistema APO solo esta disponible en Windows."))
}

#[cfg(not(target_os = "windows"))]
pub fn desinstalar_apo() -> Result<(), String> {
    Err(String::from("El subsistema APO solo esta disponible en Windows."))
}

#[cfg(not(target_os = "windows"))]
pub fn solicitar_elevacion_administrador(_arg: &str) -> bool {
    false
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;

    type Hkey = usize;"""
        texto = texto.replace(ancla, stubs_linux, 1)
        texto = texto.replace("pub struct EstadoInstalacionApo {", "struct _EstadoInstalacionApoWin {", 1)
        texto += "\n}\n#[cfg(target_os = \"windows\")]\npub use windows_impl::*;\n"
        apo_path.write_text(texto, encoding="utf-8")
        print("[+] instalador_apo.rs acondicionado para Linux.")

# 4. Parchear ventana_nativa_win32.rs
win32_path = Path("src/ventana_nativa_win32.rs")
if win32_path.exists():
    texto = win32_path.read_text(encoding="utf-8")
    if "mod windows_impl" not in texto:
        ancla = "const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;"
        stubs_linux = """#[cfg(not(target_os = "windows"))]
pub fn ejecutar_ventana_nativa_poweramp(
    _cadena_dsp: Arc<Mutex<CadenaMaestraDsp>>,
    _telemetria: Arc<TelemetriaCompartidaVentana>,
    _enrutamiento: Arc<Mutex<EnrutamientoWasapiEstado>>,
    _directorio_base: PathBuf,
) -> Result<(), String> {
    Err(String::from("La ventana nativa Win32 solo esta disponible en Windows. En Linux se utiliza la interfaz nativa Iced con PipeWire."))
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;

    const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;"""
        texto = texto.replace(ancla, stubs_linux, 1)
        texto += "\n}\n#[cfg(target_os = \"windows\")]\npub use windows_impl::ejecutar_ventana_nativa_poweramp;\n"
        win32_path.write_text(texto, encoding="utf-8")
        print("[+] ventana_nativa_win32.rs acondicionado para Linux.")
