//! Interfaz Grafica de Escritorio 100% Nativa en Rust (Win32 + GDI Doble Bufer a 60 FPS).
//! Reconstruida directamente a partir de los layouts AXML extraidos con REA de `Equalizador Poweramp.apk`:
//! - `activity_peq_main.xml` (`PeqTopNavSceneFastLayout`, `MilkWidget`, `Navbar`)
//! - `merge_eq_layout.xml` (Pestanas superiores y botones maestros `Equ`, `Tone`, `Limit`, `Parametric`)
//! - `merge_eq_equalizer_tab_common.xml` (`MilkFRS` analizador FFT de 64 barras, `FrequencyResponseScroller`
//!   curva Biquad interactiva con nodos arrastrables, `EqKnobsList`, y 2 perillas `DSPToneRoundKnobLayout` Graves/Agudos)
//! - `merge_eq_vol_tab.xml` + `peq_compr.xml` (Perillas de Balance L/R, Expansion Estereo, Tempo, Mono y Compresor Dinamico)
//! - `merge_eq_reverb_tab.xml` (2 filas de perillas de Reverberacion Schroeder-Moorer / Freeverb + Presets de Sala)
//! - `merge_navbar.xml` (Barra inferior de 4 escenas + Buscador instantaneo de 5,467 audifonos en `auto_eq.db` + Enrutador WASAPI).

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::analizador_espectro::NUMERO_BARRAS_ESPECTRO;
use crate::apo_com_windows::{guardar_estado_binario_ipc, ruta_estado_ipc_global, CLSID_APO_SOBERANO_TEXTO};
use crate::cadena_maestra_dsp::CadenaMaestraDsp;
use crate::captura_wasapi_loopback::{enumerar_dispositivos_salida_windows, DispositivoAudioWindows};
use crate::cargador_presets::{
    buscar_presets_autoeq_en_archivo, cargar_preset_autoeq_por_consulta,
    cargar_preset_json_desde_archivo, exportar_archivo_apo, ResumenPresetAutoEq,
};
use crate::tipos_audio::{
    BandaParametrica, EstadoCompletoDsp, ParametrosReverberacion, TipoFiltro,
};

/// Telemetria atomica libre de bloqueos compartida entre el hilo de audio WASAPI y la ventana nativa.
pub struct TelemetriaCompartidaVentana {
    pub barras_espectro: [AtomicU32; NUMERO_BARRAS_ESPECTRO],
    pub reduccion_compresor_db: AtomicU32,
    pub pico_salida_db: AtomicU32,
    pub captura_wasapi_activa: AtomicBool,
    pub render_directo_activo: AtomicBool,
    pub frecuencia_actual_hz: AtomicU32,
    pub ejecutando: AtomicBool,
}

impl Default for TelemetriaCompartidaVentana {
    fn default() -> Self {
        Self::nueva()
    }
}

impl TelemetriaCompartidaVentana {
    pub fn nueva() -> Self {
        Self {
            barras_espectro: std::array::from_fn(|_| AtomicU32::new(0.0_f32.to_bits())),
            reduccion_compresor_db: AtomicU32::new(0.0_f32.to_bits()),
            pico_salida_db: AtomicU32::new((-96.0_f32).to_bits()),
            captura_wasapi_activa: AtomicBool::new(false),
            render_directo_activo: AtomicBool::new(false),
            frecuencia_actual_hz: AtomicU32::new(48000),
            ejecutando: AtomicBool::new(true),
        }
    }

    #[inline]
    pub fn publicar_espectro(
        &self,
        barras: &[f32; NUMERO_BARRAS_ESPECTRO],
        reduccion_db: f32,
        pico_db: f32,
    ) {
        for (i, &valor) in barras.iter().enumerate() {
            self.barras_espectro[i].store(valor.to_bits(), Ordering::Relaxed);
        }
        self.reduccion_compresor_db
            .store(reduccion_db.to_bits(), Ordering::Relaxed);
        self.pico_salida_db
            .store(pico_db.to_bits(), Ordering::Relaxed);
    }

    pub fn leer_espectro(&self) -> ([f32; NUMERO_BARRAS_ESPECTRO], f32, f32) {
        let mut barras = [0.0_f32; NUMERO_BARRAS_ESPECTRO];
        for (i, atomico) in self.barras_espectro.iter().enumerate() {
            barras[i] = f32::from_bits(atomico.load(Ordering::Relaxed));
        }
        let reduccion = f32::from_bits(self.reduccion_compresor_db.load(Ordering::Relaxed));
        let pico = f32::from_bits(self.pico_salida_db.load(Ordering::Relaxed));
        (barras, reduccion, pico)
    }
}

/// Configuracion de enrutamiento de dispositivos WASAPI compartida con el hilo de audio.
#[derive(Debug, Clone)]
pub struct EnrutamientoWasapiEstado {
    pub lista_dispositivos: Vec<DispositivoAudioWindows>,
    pub id_dispositivo_captura: Option<String>,
    pub id_dispositivo_salida_directa: Option<String>,
    pub nombre_dispositivo_captura: String,
    pub nombre_dispositivo_salida: String,
    pub usar_salida_directa_wasapi: bool,
    pub solicitar_reinicio_audio: bool,
}

impl Default for EnrutamientoWasapiEstado {
    fn default() -> Self {
        let lista = enumerar_dispositivos_salida_windows().unwrap_or_default();
        let predeterminado = lista
            .iter()
            .find(|d| d.es_predeterminado)
            .or_else(|| lista.first())
            .cloned();

        let (id_cap, nom_cap) = match predeterminado {
            Some(ref d) => (Some(d.identificador.clone()), d.nombre_amigable.clone()),
            None => (None, String::from("Salida Predeterminada de Windows")),
        };

        Self {
            lista_dispositivos: lista,
            id_dispositivo_captura: id_cap.clone(),
            id_dispositivo_salida_directa: id_cap,
            nombre_dispositivo_captura: nom_cap.clone(),
            nombre_dispositivo_salida: nom_cap,
            usar_salida_directa_wasapi: false,
            solicitar_reinicio_audio: false,
        }
    }
}

// ============================================================================
// DEFINICIONES FFI WIN32 Y GDI (user32.dll / gdi32.dll / kernel32.dll)
// ============================================================================

const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
const WS_VISIBLE: u32 = 0x10000000;
const CS_HREDRAW: u32 = 0x0002;
const CS_VREDRAW: u32 = 0x0001;
const CS_OWNDC: u32 = 0x0020;
const IDC_ARROW: *const u16 = 32512 as *const u16;
const WM_DESTROY: u32 = 0x0002;
const WM_SIZE: u32 = 0x0005;
const WM_PAINT: u32 = 0x000F;
const WM_CLOSE: u32 = 0x0010;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_KEYDOWN: u32 = 0x0100;
const WM_CHAR: u32 = 0x0102;
const WM_TIMER: u32 = 0x0113;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONDOWN: u32 = 0x0204;
const WM_MOUSEWHEEL: u32 = 0x020A;

const DIB_RGB_COLORS: u32 = 0;
const BI_RGB: u32 = 0;
const SRCCOPY: u32 = 0x00CC0020;
const TRANSPARENT: i32 = 1;
const CLEARTYPE_QUALITY: u32 = 5;
const DEFAULT_CHARSET: u32 = 1;
const OUT_DEFAULT_PRECIS: u32 = 0;
const CLIP_DEFAULT_PRECIS: u32 = 0;
const DEFAULT_PITCH: u32 = 0;

const DT_LEFT: u32 = 0x00000000;
const DT_CENTER: u32 = 0x00000001;
const DT_RIGHT: u32 = 0x00000002;
const DT_VCENTER: u32 = 0x00000004;
const DT_SINGLELINE: u32 = 0x00000020;
const DT_NOPREFIX: u32 = 0x00000800;

const ID_TEMPORIZADOR_60FPS: usize = 1001;

#[repr(C)]
struct WndClassExW {
    cb_size: u32,
    style: u32,
    lpfn_wnd_proc: unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: *mut c_void,
    h_icon: *mut c_void,
    h_cursor: *mut c_void,
    hbr_background: *mut c_void,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
    h_icon_sm: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RectWin {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct Msg {
    hwnd: *mut c_void,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    pt: Point,
}

#[repr(C)]
struct PaintStruct {
    hdc: *mut c_void,
    f_erase: i32,
    rc_paint: RectWin,
    f_restore: i32,
    f_inc_update: i32,
    rgb_reserved: [u8; 32],
}

#[repr(C)]
struct BitmapInfoHeader {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[repr(C)]
struct BitmapInfo {
    bmi_header: BitmapInfoHeader,
    bmi_colors: [u32; 1],
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lp_module_name: *const u16) -> *mut c_void;
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassExW(lpwcx: *const WndClassExW) -> u16;
    fn CreateWindowExW(
        dw_ex_style: u32,
        lp_class_name: *const u16,
        lp_window_name: *const u16,
        dw_style: u32,
        x: i32,
        y: i32,
        n_width: i32,
        n_height: i32,
        h_wnd_parent: *mut c_void,
        h_menu: *mut c_void,
        h_instance: *mut c_void,
        lp_param: *mut c_void,
    ) -> *mut c_void;
    fn DefWindowProcW(hwnd: *mut c_void, msg: u32, w_param: usize, l_param: isize) -> isize;
    fn ShowWindow(hwnd: *mut c_void, n_cmd_show: i32) -> i32;
    fn UpdateWindow(hwnd: *mut c_void) -> i32;
    fn GetMessageW(lp_msg: *mut Msg, hwnd: *mut c_void, min: u32, max: u32) -> i32;
    fn TranslateMessage(lp_msg: *const Msg) -> i32;
    fn DispatchMessageW(lp_msg: *const Msg) -> isize;
    fn PostQuitMessage(n_exit_code: i32);
    fn LoadCursorW(h_instance: *mut c_void, lp_cursor_name: *const u16) -> *mut c_void;
    fn SetTimer(hwnd: *mut c_void, n_id_event: usize, u_elapse: u32, lp_timer_func: usize) -> usize;
    fn InvalidateRect(hwnd: *mut c_void, lp_rect: *const RectWin, b_erase: i32) -> i32;
    fn BeginPaint(hwnd: *mut c_void, lp_paint: *mut PaintStruct) -> *mut c_void;
    fn EndPaint(hwnd: *mut c_void, lp_paint: *const PaintStruct) -> i32;
    fn GetClientRect(hwnd: *mut c_void, lp_rect: *mut RectWin) -> i32;
    fn SetCapture(hwnd: *mut c_void) -> *mut c_void;
    fn ReleaseCapture() -> i32;
    fn DrawTextW(
        hdc: *mut c_void,
        lpch_text: *const u16,
        cch_text: i32,
        lprc: *mut RectWin,
        format: u32,
    ) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(hdc: *mut c_void) -> *mut c_void;
    fn DeleteDC(hdc: *mut c_void) -> i32;
    fn SelectObject(hdc: *mut c_void, h: *mut c_void) -> *mut c_void;
    fn DeleteObject(ho: *mut c_void) -> i32;
    fn CreateDIBSection(
        hdc: *mut c_void,
        pbmi: *const BitmapInfo,
        usage: u32,
        ppv_bits: *mut *mut u32,
        h_section: *mut c_void,
        offset: u32,
    ) -> *mut c_void;
    fn BitBlt(
        hdc: *mut c_void,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        hdc_src: *mut c_void,
        x1: i32,
        y1: i32,
        rop: u32,
    ) -> i32;
    fn SetBkMode(hdc: *mut c_void, mode: i32) -> i32;
    fn SetTextColor(hdc: *mut c_void, color: u32) -> u32;
    fn CreateFontW(
        c_height: i32,
        c_width: i32,
        c_escapement: i32,
        c_orientation: i32,
        c_weight: i32,
        b_italic: u32,
        b_underline: u32,
        b_strike_out: u32,
        i_char_set: u32,
        i_out_precision: u32,
        i_clip_precision: u32,
        i_quality: u32,
        i_pitch_and_family: u32,
        psz_face_name: *const u16,
    ) -> *mut c_void;
}

fn a_utf16_nulo(texto: &str) -> Vec<u16> {
    let mut v: Vec<u16> = texto.encode_utf16().collect();
    v.push(0);
    v
}

// ============================================================================
// MOTOR DE RENDERIZADO DE PIXELES 2D ANTIALIAS + GDI CLEARTYPE
// ============================================================================

/// Paleta cromatica Aurora + Poweramp Dark Skin (`peq_builtin_skins.xml`).
const COLOR_OBSIDIANA_FONDO: u32 = 0x07090E;
const COLOR_PANEL_SUPERIOR: u32 = 0x0D111A;
const COLOR_PANEL_TARJETA: u32 = 0x111724;
const COLOR_PANEL_ELEVADO: u32 = 0x171F30;
const COLOR_BORDE_SUTIL: u32 = 0x222D42;
const COLOR_AMBAR_SUNLIGHT: u32 = 0xF59E0B;
const COLOR_CHERRY_NEON: u32 = 0xF43F5E;
const COLOR_CIAN_ELECTRICO: u32 = 0x06B6D4;
const COLOR_VERDE_ESMERALDA: u32 = 0x10B981;
const COLOR_TEXTO_BLANCO: u32 = 0xF8FAFC;
const COLOR_TEXTO_GRIS: u32 = 0x94A3B8;
const COLOR_TEXTO_ATENUADO: u32 = 0x64748B;

const COLORES_BANDAS_POWERAMP: [u32; 10] = [
    0xF43F5E, // Cherry
    0xF97316, // Naranja Lava
    0xF59E0B, // Ambar Sunlight
    0xEAB308, // Oro
    0x10B981, // Esmeralda
    0x14B8A6, // Turquesa
    0x06B6D4, // Cian
    0x3B82F6, // Azul Cobalto
    0xEC4899, // Rosa Coral
    0xF43F5E, // Cherry
];

#[derive(Clone, Copy)]
struct RectanguloUi {
    x: i32,
    y: i32,
    ancho: i32,
    alto: i32,
}

impl RectanguloUi {
    fn contiene(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < (self.x + self.ancho) && py >= self.y && py < (self.y + self.alto)
    }
}

/// Lienzo de pixeles ARGB en memoria con soporte para primitivas vectoriales suaves y texto GDI.
struct LienzoDobleBufer {
    hdc_memoria: *mut c_void,
    hbitmap_dib: *mut c_void,
    puntero_pixeles: *mut u32,
    ancho: i32,
    alto: i32,
    fuente_titulo: *mut c_void,
    fuente_subtitulo: *mut c_void,
    fuente_normal: *mut c_void,
    fuente_mono: *mut c_void,
    fuente_mono_pequena: *mut c_void,
}

impl LienzoDobleBufer {
    unsafe fn nuevo(ancho: i32, alto: i32) -> Self {
        let hdc_memoria = CreateCompatibleDC(ptr::null_mut());
        let _ = SetBkMode(hdc_memoria, TRANSPARENT);

        let nombre_titulo = a_utf16_nulo("Space Grotesk");
        let nombre_ui = a_utf16_nulo("Inter");
        let nombre_mono = a_utf16_nulo("JetBrains Mono");

        let fuente_titulo = CreateFontW(
            -20,
            0,
            0,
            0,
            700,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH,
            nombre_titulo.as_ptr(),
        );
        let fuente_subtitulo = CreateFontW(
            -14,
            0,
            0,
            0,
            700,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH,
            nombre_ui.as_ptr(),
        );
        let fuente_normal = CreateFontW(
            -13,
            0,
            0,
            0,
            500,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH,
            nombre_ui.as_ptr(),
        );
        let fuente_mono = CreateFontW(
            -13,
            0,
            0,
            0,
            600,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH,
            nombre_mono.as_ptr(),
        );
        let fuente_mono_pequena = CreateFontW(
            -11,
            0,
            0,
            0,
            500,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH,
            nombre_mono.as_ptr(),
        );

        let mut lienzo = Self {
            hdc_memoria,
            hbitmap_dib: ptr::null_mut(),
            puntero_pixeles: ptr::null_mut(),
            ancho: 0,
            alto: 0,
            fuente_titulo,
            fuente_subtitulo,
            fuente_normal,
            fuente_mono,
            fuente_mono_pequena,
        };
        lienzo.redimensionar(ancho.max(640), alto.max(480));
        lienzo
    }

    unsafe fn redimensionar(&mut self, nuevo_ancho: i32, nuevo_alto: i32) {
        let w = nuevo_ancho.max(320);
        let h = nuevo_alto.max(240);
        if w == self.ancho && h == self.alto && !self.puntero_pixeles.is_null() {
            return;
        }
        if !self.hbitmap_dib.is_null() {
            DeleteObject(self.hbitmap_dib);
            self.hbitmap_dib = ptr::null_mut();
            self.puntero_pixeles = ptr::null_mut();
        }

        let info = BitmapInfo {
            bmi_header: BitmapInfoHeader {
                bi_size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                bi_width: w,
                bi_height: -h, // Top-down DIB (origen en esquina superior izquierda)
                bi_planes: 1,
                bi_bit_count: 32,
                bi_compression: BI_RGB,
                bi_size_image: 0,
                bi_x_pels_per_meter: 0,
                bi_y_pels_per_meter: 0,
                bi_clr_used: 0,
                bi_clr_important: 0,
            },
            bmi_colors: [0],
        };

        let mut bits: *mut u32 = ptr::null_mut();
        let hbmp = CreateDIBSection(
            self.hdc_memoria,
            &info,
            DIB_RGB_COLORS,
            &mut bits,
            ptr::null_mut(),
            0,
        );
        if !hbmp.is_null() && !bits.is_null() {
            SelectObject(self.hdc_memoria, hbmp);
            self.hbitmap_dib = hbmp;
            self.puntero_pixeles = bits;
            self.ancho = w;
            self.alto = h;
        }
    }

    #[inline(always)]
    fn mezclar_pixel(&mut self, x: i32, y: i32, color_rgb: u32, alfa: u8) {
        if x < 0 || y < 0 || x >= self.ancho || y >= self.alto || self.puntero_pixeles.is_null() {
            return;
        }
        unsafe {
            let idx = (y * self.ancho + x) as usize;
            let ptr = self.puntero_pixeles.add(idx);
            if alfa == 255 {
                *ptr = color_rgb;
                return;
            }
            if alfa == 0 {
                return;
            }
            let fondo = *ptr;
            let a = alfa as u32;
            let inv_a = 255 - a;

            let r_src = (color_rgb >> 16) & 0xFF;
            let g_src = (color_rgb >> 8) & 0xFF;
            let b_src = color_rgb & 0xFF;

            let r_bg = (fondo >> 16) & 0xFF;
            let g_bg = (fondo >> 8) & 0xFF;
            let b_bg = fondo & 0xFF;

            let r = (r_src * a + r_bg * inv_a) / 255;
            let g = (g_src * a + g_bg * inv_a) / 255;
            let b = (b_src * a + b_bg * inv_a) / 255;

            *ptr = (r << 16) | (g << 8) | b;
        }
    }

    fn limpiar(&mut self, color_rgb: u32) {
        if self.puntero_pixeles.is_null() {
            return;
        }
        unsafe {
            let total = (self.ancho * self.alto) as usize;
            let slice = std::slice::from_raw_parts_mut(self.puntero_pixeles, total);
            slice.fill(color_rgb);
        }
    }

    fn llenar_rectangulo(&mut self, r: RectanguloUi, color_rgb: u32, alfa: u8) {
        let x0 = r.x.max(0);
        let y0 = r.y.max(0);
        let x1 = (r.x + r.ancho).min(self.ancho);
        let y1 = (r.y + r.alto).min(self.alto);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        if alfa == 255 {
            unsafe {
                for y in y0..y1 {
                    let fila = self.puntero_pixeles.add((y * self.ancho + x0) as usize);
                    let segmento = std::slice::from_raw_parts_mut(fila, (x1 - x0) as usize);
                    segmento.fill(color_rgb);
                }
            }
        } else {
            for y in y0..y1 {
                for x in x0..x1 {
                    self.mezclar_pixel(x, y, color_rgb, alfa);
                }
            }
        }
    }

    fn dibujar_caja_redondeada(
        &mut self,
        r: RectanguloUi,
        radio: i32,
        color_relleno: u32,
        alfa_relleno: u8,
        color_borde: Option<u32>,
    ) {
        let rad = radio.clamp(0, (r.ancho.min(r.alto)) / 2);
        for dy in 0..r.alto {
            let py = r.y + dy;
            for dx in 0..r.ancho {
                let px = r.x + dx;
                let mut dentro = true;
                let mut en_borde = false;

                let cx = if dx < rad {
                    rad
                } else if dx >= r.ancho - rad {
                    r.ancho - rad - 1
                } else {
                    dx
                };
                let cy = if dy < rad {
                    rad
                } else if dy >= r.alto - rad {
                    r.alto - rad - 1
                } else {
                    dy
                };

                if (dx < rad || dx >= r.ancho - rad) && (dy < rad || dy >= r.alto - rad) {
                    let dist2 = (dx - cx) * (dx - cx) + (dy - cy) * (dy - cy);
                    if dist2 > rad * rad {
                        dentro = false;
                    } else if dist2 >= (rad - 1).max(0) * (rad - 1).max(0) {
                        en_borde = true;
                    }
                } else if dx == 0 || dx == r.ancho - 1 || dy == 0 || dy == r.alto - 1 {
                    en_borde = true;
                }

                if dentro {
                    if en_borde {
                        if let Some(cb) = color_borde {
                            self.mezclar_pixel(px, py, cb, 255);
                        } else {
                            self.mezclar_pixel(px, py, color_relleno, alfa_relleno);
                        }
                    } else {
                        self.mezclar_pixel(px, py, color_relleno, alfa_relleno);
                    }
                }
            }
        }
    }

    fn dibujar_linea(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, grosor: f32, color_rgb: u32, alfa: u8) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let pasos = dx.abs().max(dy.abs()).ceil() as i32;
        if pasos <= 0 {
            self.mezclar_pixel(x0 as i32, y0 as i32, color_rgb, alfa);
            return;
        }
        let radio = (grosor * 0.5).max(0.5);
        let r_entero = radio.ceil() as i32;
        for i in 0..=pasos {
            let t = (i as f32) / (pasos as f32);
            let px = x0 + dx * t;
            let py = y0 + dy * t;
            let ix = px.round() as i32;
            let iy = py.round() as i32;
            for oy in -r_entero..=r_entero {
                for ox in -r_entero..=r_entero {
                    let dist = (((ox as f32) + (ix as f32 - px)).powi(2)
                        + ((oy as f32) + (iy as f32 - py)).powi(2))
                    .sqrt();
                    if dist <= radio + 0.6 {
                        let factor = (1.0 - (dist - radio).max(0.0) / 0.8).clamp(0.0, 1.0);
                        let a_final = ((alfa as f32) * factor) as u8;
                        self.mezclar_pixel(ix + ox, iy + oy, color_rgb, a_final);
                    }
                }
            }
        }
    }

    fn dibujar_circulo_relleno(&mut self, cx: f32, cy: f32, radio: f32, color_rgb: u32, alfa: u8) {
        let r_int = (radio + 1.5).ceil() as i32;
        let icx = cx.round() as i32;
        let icy = cy.round() as i32;
        for dy in -r_int..=r_int {
            for dx in -r_int..=r_int {
                let px = icx + dx;
                let py = icy + dy;
                let dist = (((px as f32) - cx).powi(2) + ((py as f32) - cy).powi(2)).sqrt();
                if dist <= radio + 0.7 {
                    let cobertura = (1.0 - (dist - radio).max(0.0)).clamp(0.0, 1.0);
                    self.mezclar_pixel(px, py, color_rgb, ((alfa as f32) * cobertura) as u8);
                }
            }
        }
    }

    fn dibujar_arco_perilla(
        &mut self,
        cx: f32,
        cy: f32,
        radio: f32,
        angulo_inicio_rad: f32,
        angulo_fin_rad: f32,
        grosor: f32,
        color_rgb: u32,
        alfa: u8,
    ) {
        let pasos = ((angulo_fin_rad - angulo_inicio_rad).abs() * radio * 1.4)
            .ceil()
            .max(12.0) as i32;
        let mut ant_x = cx + radio * angulo_inicio_rad.cos();
        let mut ant_y = cy + radio * angulo_inicio_rad.sin();
        for i in 1..=pasos {
            let t = (i as f32) / (pasos as f32);
            let ang = angulo_inicio_rad + (angulo_fin_rad - angulo_inicio_rad) * t;
            let nx = cx + radio * ang.cos();
            let ny = cy + radio * ang.sin();
            self.dibujar_linea(ant_x, ant_y, nx, ny, grosor, color_rgb, alfa);
            ant_x = nx;
            ant_y = ny;
        }
    }

    unsafe fn dibujar_texto(
        &mut self,
        texto: &str,
        r: RectanguloUi,
        color_rgb: u32,
        fuente: *mut c_void,
        alineacion: u32,
    ) {
        if texto.is_empty() || self.hdc_memoria.is_null() {
            return;
        }
        SelectObject(self.hdc_memoria, fuente);
        // GDI COLORREF usa orden 0x00BBGGRR
        let r_c = (color_rgb >> 16) & 0xFF;
        let g_c = (color_rgb >> 8) & 0xFF;
        let b_c = color_rgb & 0xFF;
        let colorref = (b_c << 16) | (g_c << 8) | r_c;
        SetTextColor(self.hdc_memoria, colorref);

        let utf16: Vec<u16> = texto.encode_utf16().collect();
        let mut rc = RectWin {
            left: r.x,
            top: r.y,
            right: r.x + r.ancho,
            bottom: r.y + r.alto,
        };
        DrawTextW(
            self.hdc_memoria,
            utf16.as_ptr(),
            utf16.len() as i32,
            &mut rc,
            alineacion | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
        );
    }
}

impl Drop for LienzoDobleBufer {
    fn drop(&mut self) {
        unsafe {
            if !self.hbitmap_dib.is_null() {
                DeleteObject(self.hbitmap_dib);
            }
            if !self.fuente_titulo.is_null() {
                DeleteObject(self.fuente_titulo);
            }
            if !self.fuente_subtitulo.is_null() {
                DeleteObject(self.fuente_subtitulo);
            }
            if !self.fuente_normal.is_null() {
                DeleteObject(self.fuente_normal);
            }
            if !self.fuente_mono.is_null() {
                DeleteObject(self.fuente_mono);
            }
            if !self.fuente_mono_pequena.is_null() {
                DeleteObject(self.fuente_mono_pequena);
            }
            if !self.hdc_memoria.is_null() {
                DeleteDC(self.hdc_memoria);
            }
        }
    }
}

// ============================================================================
// ESTADO DE INTERACCION DE LA INTERFAZ NATIVA POWERAMP
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
enum EscenaPoweramp {
    EcualizadorYTono = 0,
    EspacioYCompresor = 1,
    Reverberacion = 2,
    AutoEqYDispositivos = 3,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ControlArrastrado {
    Ninguno,
    NodoBanda(usize),
    DeslizadorPreamp,
    PerillaGraves,
    PerillaAgudos,
    PerillaBalance,
    PerillaExpansionEstereo,
    PerillaCompUmbral,
    PerillaCompProporcion,
    PerillaCompAtaque,
    PerillaCompLiberacion,
    PerillaCompSidechain,
    PerillaCompMaquillaje,
    PerillaRevTamano,
    PerillaRevPreretardo,
    PerillaRevAmortiguacion,
    PerillaRevFiltroTonal,
    PerillaRevMezclado,
}

struct ZonaInteractiva {
    rect: RectanguloUi,
    identificador: AccionZona,
}

#[derive(Clone)]
enum AccionZona {
    CambiarEscena(EscenaPoweramp),
    AlternarEqActivo,
    AlternarTonoActivo,
    AlternarLimitador,
    AlternarEnrutadorDirectoWasapi,
    AgregarBandaParametrica,
    EliminarBandaSeleccionada,
    ReiniciarPlano,
    SeleccionarBanda(usize),
    AlternarBandaActiva(usize),
    CiclarTipoFiltroBanda(usize),
    AplicarPresetRapido(usize),
    AlternarModoMono,
    CentrarBalance,
    AlternarCompresorActivo,
    AlternarReverbActiva,
    AplicarPresetReverb(usize),
    EnfocarBuscadorAutoEq,
    CargarModeloAutoEq(String),
    SeleccionarDispositivoCaptura(String, String),
    SeleccionarDispositivoSalidaDirecta(String, String),
    RefrescarDispositivosWasapi,
    IniciarArrastre(ControlArrastrado, f64),
}

struct ContextoVentanaPoweramp {
    lienzo: LienzoDobleBufer,
    cadena_dsp: Arc<Mutex<CadenaMaestraDsp>>,
    telemetria: Arc<TelemetriaCompartidaVentana>,
    enrutamiento: Arc<Mutex<EnrutamientoWasapiEstado>>,
    directorio_base: PathBuf,
    ruta_ipc_binario: PathBuf,
    escena_actual: EscenaPoweramp,
    banda_seleccionada: usize,
    control_arrastrado: ControlArrastrado,
    raton_inicio_y: i32,
    valor_inicial_arrastre: f64,
    rect_lienzo_curva: RectanguloUi,
    rect_fader_preamp: RectanguloUi,
    consulta_autoeq: String,
    buscador_autoeq_enfocado: bool,
    resultados_autoeq: Vec<ResumenPresetAutoEq>,
    mensaje_estado_barra: String,
    zonas_clic: Vec<ZonaInteractiva>,
}

static mut PUNTERO_CONTEXTO_GLOBAL: *mut ContextoVentanaPoweramp = ptr::null_mut();

impl ContextoVentanaPoweramp {
    unsafe fn nuevo(
        cadena_dsp: Arc<Mutex<CadenaMaestraDsp>>,
        telemetria: Arc<TelemetriaCompartidaVentana>,
        enrutamiento: Arc<Mutex<EnrutamientoWasapiEstado>>,
        directorio_base: PathBuf,
    ) -> Self {
        let ruta_ipc_binario = ruta_estado_ipc_global(Some(&directorio_base));
        let ruta_db = localizar_autoeq_db(&directorio_base);
        let resultados_iniciales =
            buscar_presets_autoeq_en_archivo(&ruta_db, "JBL", 30).unwrap_or_default();

        Self {
            lienzo: LienzoDobleBufer::nuevo(1360, 860),
            cadena_dsp,
            telemetria,
            enrutamiento,
            directorio_base,
            ruta_ipc_binario,
            escena_actual: EscenaPoweramp::EcualizadorYTono,
            banda_seleccionada: 0,
            control_arrastrado: ControlArrastrado::Ninguno,
            raton_inicio_y: 0,
            valor_inicial_arrastre: 0.0,
            rect_lienzo_curva: RectanguloUi {
                x: 20,
                y: 84,
                ancho: 1320,
                alto: 420,
            },
            rect_fader_preamp: RectanguloUi {
                x: 42,
                y: 560,
                ancho: 32,
                alto: 170,
            },
            consulta_autoeq: String::from("JBL"),
            buscador_autoeq_enfocado: false,
            resultados_autoeq: resultados_iniciales,
            mensaje_estado_barra: String::from(
                "Motor DSP Rust 64-bit Activo | Layouts extraidos de Equalizador Poweramp.apk via REA",
            ),
            zonas_clic: Vec::with_capacity(128),
        }
    }

    /// Persiste inmediatamente cualquier cambio de parametros tanto en el bloque binario IPC (`estado_dsp_activo.bin`)
    /// para nuestro APO COM como en `apo_config.txt`.
    fn sincronizar_persistencia_dsp(&self, estado: &EstadoCompletoDsp) {
        let _ = guardar_estado_binario_ipc(&self.ruta_ipc_binario, estado);
        let _ = exportar_archivo_apo(&self.directorio_base.join("apo_config.txt"), estado);
    }

    fn ejecutar_busqueda_autoeq(&mut self) {
        let ruta_db = localizar_autoeq_db(&self.directorio_base);
        if let Ok(lista) = buscar_presets_autoeq_en_archivo(&ruta_db, &self.consulta_autoeq, 24) {
            self.resultados_autoeq = lista;
        }
    }

    // ========================================================================
    // RENDERIZADO COMPLETO DE LA VENTANA A 60 FPS
    // ========================================================================

    unsafe fn renderizar_cuadro_completo(&mut self) {
        self.zonas_clic.clear();
        self.lienzo.limpiar(COLOR_OBSIDIANA_FONDO);

        let estado = if let Ok(c) = self.cadena_dsp.lock() {
            c.obtener_estado().clone()
        } else {
            return;
        };

        let (barras_fft, reduccion_comp_db, pico_salida_db) = self.telemetria.leer_espectro();
        let wasapi_activo = self.telemetria.captura_wasapi_activa.load(Ordering::Relaxed);
        let fs_actual = self.telemetria.frecuencia_actual_hz.load(Ordering::Relaxed);

        let info_enrutamiento = if let Ok(enr) = self.enrutamiento.lock() {
            enr.clone()
        } else {
            EnrutamientoWasapiEstado::default()
        };

        let w = self.lienzo.ancho;
        let h = self.lienzo.alto;

        // 1. BARRA SUPERIOR POWERAMP (`PeqTopNavSceneFastLayout` / `merge_eq_layout.xml`)
        self.dibujar_barra_superior(w, &estado, &info_enrutamiento, wasapi_activo, fs_actual);

        // 2. CONTENIDO DE LA ESCENA ACTIVA
        let area_contenido = RectanguloUi {
            x: 16,
            y: 74,
            ancho: w - 32,
            alto: h - 138,
        };

        match self.escena_actual {
            EscenaPoweramp::EcualizadorYTono => {
                self.dibujar_escena_ecualizador_y_tono(area_contenido, &estado, &barras_fft);
            }
            EscenaPoweramp::EspacioYCompresor => {
                self.dibujar_escena_espacio_y_compresor(
                    area_contenido,
                    &estado,
                    reduccion_comp_db,
                    pico_salida_db,
                );
            }
            EscenaPoweramp::Reverberacion => {
                self.dibujar_escena_reverberacion(area_contenido, &estado);
            }
            EscenaPoweramp::AutoEqYDispositivos => {
                self.dibujar_escena_autoeq_y_dispositivos(
                    area_contenido,
                    &estado,
                    &info_enrutamiento,
                );
            }
        }

        // 3. BARRA DE NAVEGACION INFERIOR (`merge_navbar.xml`)
        self.dibujar_barra_inferior_navbar(w, h, pico_salida_db, reduccion_comp_db);
    }

    unsafe fn dibujar_barra_superior(
        &mut self,
        ancho_ventana: i32,
        estado: &EstadoCompletoDsp,
        enrutamiento: &EnrutamientoWasapiEstado,
        wasapi_activo: bool,
        fs_actual: u32,
    ) {
        let rect_barra = RectanguloUi {
            x: 0,
            y: 0,
            ancho: ancho_ventana,
            alto: 66,
        };
        self.lienzo
            .llenar_rectangulo(rect_barra, COLOR_PANEL_SUPERIOR, 255);
        self.lienzo.dibujar_linea(
            0.0,
            65.0,
            ancho_ventana as f32,
            65.0,
            1.0,
            COLOR_BORDE_SUTIL,
            255,
        );

        // Punto luminoso de estado + Titulo Poweramp Equalizer
        let color_led = if wasapi_activo {
            COLOR_VERDE_ESMERALDA
        } else {
            COLOR_AMBAR_SUNLIGHT
        };
        self.lienzo
            .dibujar_circulo_relleno(24.0, 24.0, 6.0, color_led, 255);

        self.lienzo.dibujar_texto(
            "POWERAMP EQUALIZER",
            RectanguloUi {
                x: 38,
                y: 8,
                ancho: 250,
                alto: 28,
            },
            COLOR_TEXTO_BLANCO,
            self.lienzo.fuente_titulo,
            DT_LEFT,
        );

        let subtitulo_motor = format!(
            "DSP RUST 64-BIT  |  {} Hz  |  SALIDA: {}",
            fs_actual,
            truncar_texto(&enrutamiento.nombre_dispositivo_captura, 34)
        );
        self.lienzo.dibujar_texto(
            &subtitulo_motor,
            RectanguloUi {
                x: 38,
                y: 36,
                ancho: 430,
                alto: 20,
            },
            COLOR_AMBAR_SUNLIGHT,
            self.lienzo.fuente_mono_pequena,
            DT_LEFT,
        );

        // Selector de Preset Activo (`EqPresetPopupListLayout`)
        let rect_preset = RectanguloUi {
            x: 475,
            y: 14,
            ancho: 230,
            alto: 38,
        };
        self.lienzo.dibujar_caja_redondeada(
            rect_preset,
            8,
            COLOR_PANEL_TARJETA,
            255,
            Some(COLOR_BORDE_SUTIL),
        );
        let texto_preset = format!("PRESET: {}", truncar_texto(&estado.nombre_perfil, 20));
        self.lienzo.dibujar_texto(
            &texto_preset,
            RectanguloUi {
                x: rect_preset.x + 12,
                y: rect_preset.y,
                ancho: rect_preset.ancho - 24,
                alto: rect_preset.alto,
            },
            COLOR_CIAN_ELECTRICO,
            self.lienzo.fuente_subtitulo,
            DT_LEFT,
        );
        self.zonas_clic.push(ZonaInteractiva {
            rect: rect_preset,
            identificador: AccionZona::CambiarEscena(EscenaPoweramp::AutoEqYDispositivos),
        });

        // Botones de control maestro (`merge_eq_equalizer_buttons.xml`: EQU, TONO, LIMIT, DIRECTO, +BANDA, -BANDA, PLANO)
        let mut cursor_x = ancho_ventana - 635;
        let botones_superiores: [(&str, bool, u32, AccionZona); 7] = [
            (
                "EQU",
                estado.ecualizador_activo,
                COLOR_AMBAR_SUNLIGHT,
                AccionZona::AlternarEqActivo,
            ),
            (
                "TONO",
                estado.tono.tono_activo,
                COLOR_CIAN_ELECTRICO,
                AccionZona::AlternarTonoActivo,
            ),
            (
                "LIMIT",
                estado.limitador_activo,
                COLOR_VERDE_ESMERALDA,
                AccionZona::AlternarLimitador,
            ),
            (
                "WASAPI DIRECTO",
                enrutamiento.usar_salida_directa_wasapi,
                COLOR_CHERRY_NEON,
                AccionZona::AlternarEnrutadorDirectoWasapi,
            ),
            (
                "+ BANDA",
                false,
                COLOR_PANEL_ELEVADO,
                AccionZona::AgregarBandaParametrica,
            ),
            (
                "- BANDA",
                false,
                COLOR_PANEL_ELEVADO,
                AccionZona::EliminarBandaSeleccionada,
            ),
            (
                "PLANO",
                false,
                COLOR_PANEL_ELEVADO,
                AccionZona::ReiniciarPlano,
            ),
        ];

        for (etiqueta, activo, color_activo, accion) in botones_superiores {
            let ancho_btn = if etiqueta.len() > 8 { 126 } else { 76 };
            let r_btn = RectanguloUi {
                x: cursor_x,
                y: 15,
                ancho: ancho_btn,
                alto: 36,
            };
            let relleno = if activo {
                color_activo
            } else {
                COLOR_PANEL_TARJETA
            };
            let borde = if activo {
                color_activo
            } else {
                COLOR_BORDE_SUTIL
            };
            let color_txt = if activo {
                0x07090E
            } else {
                COLOR_TEXTO_BLANCO
            };
            self.lienzo
                .dibujar_caja_redondeada(r_btn, 7, relleno, 255, Some(borde));
            self.lienzo.dibujar_texto(
                etiqueta,
                r_btn,
                color_txt,
                self.lienzo.fuente_mono_pequena,
                DT_CENTER,
            );
            self.zonas_clic.push(ZonaInteractiva {
                rect: r_btn,
                identificador: accion,
            });
            cursor_x += ancho_btn + 8;
        }
    }

    // ========================================================================
    // ESCENA 0: ECUALIZADOR PARAMETRICO + ESPECTRO FFT + PERILLAS DE TONO
    // (`merge_eq_equalizer_tab_common.xml`)
    // ========================================================================

    unsafe fn dibujar_escena_ecualizador_y_tono(
        &mut self,
        area: RectanguloUi,
        estado: &EstadoCompletoDsp,
        barras_fft: &[f32; NUMERO_BARRAS_ESPECTRO],
    ) {
        let alto_lienzo = (area.alto as f32 * 0.56) as i32;
        let rect_visor = RectanguloUi {
            x: area.x,
            y: area.y,
            ancho: area.ancho,
            alto: alto_lienzo,
        };
        self.rect_lienzo_curva = RectanguloUi {
            x: rect_visor.x + 52,
            y: rect_visor.y + 18,
            ancho: rect_visor.ancho - 74,
            alto: rect_visor.alto - 42,
        };

        self.lienzo.dibujar_caja_redondeada(
            rect_visor,
            12,
            COLOR_PANEL_SUPERIOR,
            255,
            Some(COLOR_BORDE_SUTIL),
        );

        let rc = self.rect_lienzo_curva;

        // 1. Cuadricula logaritmica de frecuencias (20 Hz a 20 kHz) y ganancia (-18 dB a +18 dB)
        let ganancias_guia = [-18.0, -12.0, -6.0, 0.0, 6.0, 12.0, 18.0];
        for &g_db in &ganancias_guia {
            let py = ganancia_a_y(g_db, rc);
            let color_linea = if g_db == 0.0 { 0x334155 } else { 0x161F30 };
            self.lienzo.dibujar_linea(
                rc.x as f32,
                py as f32,
                (rc.x + rc.ancho) as f32,
                py as f32,
                1.0,
                color_linea,
                255,
            );
            let txt_db = if g_db > 0.0 {
                format!("+{:.0}", g_db)
            } else {
                format!("{:.0}", g_db)
            };
            self.lienzo.dibujar_texto(
                &txt_db,
                RectanguloUi {
                    x: rect_visor.x + 6,
                    y: py - 9,
                    ancho: 40,
                    alto: 18,
                },
                COLOR_TEXTO_ATENUADO,
                self.lienzo.fuente_mono_pequena,
                DT_RIGHT,
            );
        }

        let frecuencias_guia = [
            (20.0, "20"),
            (50.0, "50"),
            (100.0, "100"),
            (200.0, "200"),
            (500.0, "500"),
            (1000.0, "1k"),
            (2000.0, "2k"),
            (5000.0, "5k"),
            (10000.0, "10k"),
            (20000.0, "20k"),
        ];
        for &(f_hz, etiqueta) in &frecuencias_guia {
            let px = frecuencia_a_x(f_hz, rc);
            self.lienzo.dibujar_linea(
                px as f32,
                rc.y as f32,
                px as f32,
                (rc.y + rc.alto) as f32,
                1.0,
                0x161F30,
                255,
            );
            self.lienzo.dibujar_texto(
                etiqueta,
                RectanguloUi {
                    x: px - 20,
                    y: rc.y + rc.alto + 4,
                    ancho: 40,
                    alto: 18,
                },
                COLOR_TEXTO_ATENUADO,
                self.lienzo.fuente_mono_pequena,
                DT_CENTER,
            );
        }

        // 2. Espectro FFT en Tiempo Real de 64 Barras (`MilkFRS` / `milk_main.lua`)
        let ancho_paso_barra = (rc.ancho as f32) / (NUMERO_BARRAS_ESPECTRO as f32);
        let ancho_barra = (ancho_paso_barra * 0.72).max(3.0) as i32;
        for (idx, &mag) in barras_fft.iter().enumerate() {
            let mag_clamped = mag.clamp(0.0, 1.0);
            let alto_b = ((rc.alto as f32) * 0.90 * mag_clamped) as i32;
            if alto_b > 1 {
                let bx = rc.x + ((idx as f32) * ancho_paso_barra) as i32;
                let by = rc.y + rc.alto - alto_b;
                let color_barra = if mag_clamped > 0.78 {
                    COLOR_CHERRY_NEON
                } else if mag_clamped > 0.45 {
                    COLOR_AMBAR_SUNLIGHT
                } else {
                    COLOR_CIAN_ELECTRICO
                };
                self.lienzo.llenar_rectangulo(
                    RectanguloUi {
                        x: bx,
                        y: by,
                        ancho: ancho_barra,
                        alto: alto_b,
                    },
                    color_barra,
                    82,
                );
                // Cima brillante de la barra FFT
                self.lienzo.llenar_rectangulo(
                    RectanguloUi {
                        x: bx,
                        y: by,
                        ancho: ancho_barra,
                        alto: 2,
                    },
                    color_barra,
                    220,
                );
            }
        }

        // 3. Curva de Respuesta Biquad Combinada (`FrequencyResponseScroller` / `Coefs.smali`)
        let num_muestras_curva = 220usize;
        let mut freqs = Vec::with_capacity(num_muestras_curva);
        for i in 0..num_muestras_curva {
            let t = (i as f64) / ((num_muestras_curva - 1) as f64);
            freqs.push(20.0 * 1000.0_f64.powf(t));
        }
        let ganancias_curva = if let Ok(c) = self.cadena_dsp.lock() {
            c.evaluar_curva_respuesta_db(&freqs)
        } else {
            vec![0.0; num_muestras_curva]
        };

        let y_cero = ganancia_a_y(0.0, rc);
        let mut ant_x = rc.x as f32;
        let mut ant_y = y_cero as f32;

        for (i, (&f, &g_db)) in freqs.iter().zip(ganancias_curva.iter()).enumerate() {
            let px = frecuencia_a_x(f, rc);
            let py = ganancia_a_y(g_db, rc);

            // Relleno sombreado bajo la curva hacia la linea de 0 dB
            let y_min = py.min(y_cero);
            let y_max = py.max(y_cero);
            let ancho_col = ((rc.ancho as f32 / num_muestras_curva as f32).ceil() as i32).max(2);
            self.lienzo.llenar_rectangulo(
                RectanguloUi {
                    x: px,
                    y: y_min,
                    ancho: ancho_col,
                    alto: (y_max - y_min).max(1),
                },
                COLOR_AMBAR_SUNLIGHT,
                32,
            );

            if i > 0 {
                self.lienzo.dibujar_linea(
                    ant_x,
                    ant_y,
                    px as f32,
                    py as f32,
                    2.6,
                    COLOR_AMBAR_SUNLIGHT,
                    255,
                );
            }
            ant_x = px as f32;
            ant_y = py as f32;
        }

        // 4. Nodos Parametricos Arrastrables sobre la Curva
        for (idx, banda) in estado.bandas.iter().enumerate() {
            let nx = frecuencia_a_x(banda.frecuencia_hz, rc) as f32;
            let ny = ganancia_a_y(banda.ganancia_db, rc) as f32;
            let color_nodo = COLORES_BANDAS_POWERAMP[idx % COLORES_BANDAS_POWERAMP.len()];
            let seleccionado = idx == self.banda_seleccionada;

            if seleccionado {
                self.lienzo
                    .dibujar_circulo_relleno(nx, ny, 14.0, color_nodo, 75);
                // Guia vertical de la banda seleccionada
                self.lienzo.dibujar_linea(
                    nx,
                    rc.y as f32,
                    nx,
                    (rc.y + rc.alto) as f32,
                    1.0,
                    color_nodo,
                    110,
                );
            }

            let alfa_nodo = if banda.activo { 255 } else { 100 };
            self.lienzo
                .dibujar_circulo_relleno(nx, ny, 8.5, color_nodo, alfa_nodo);
            self.lienzo
                .dibujar_circulo_relleno(nx, ny, 3.5, 0x07090E, 255);

            // Etiqueta compacta del numero de banda sobre el nodo
            let num_txt = format!("{}", idx + 1);
            self.lienzo.dibujar_texto(
                &num_txt,
                RectanguloUi {
                    x: (nx as i32) - 14,
                    y: (ny as i32) - 24,
                    ancho: 28,
                    alto: 14,
                },
                color_nodo,
                self.lienzo.fuente_mono_pequena,
                DT_CENTER,
            );
        }

        // ====================================================================
        // SECCION INFERIOR DE ESCENA 0:
        // [PREAMP] + [TARJETAS DE BANDAS `EqKnobsList`] + [2 PERILLAS GRAVES / AGUDOS `DSPToneRoundKnobLayout`]
        // ====================================================================
        let y_inferior = rect_visor.y + rect_visor.alto + 12;
        let alto_inferior = area.alto - alto_lienzo - 12;

        // A. Columna Preamp (Izquierda)
        let rect_panel_preamp = RectanguloUi {
            x: area.x,
            y: y_inferior,
            ancho: 105,
            alto: alto_inferior,
        };
        self.lienzo.dibujar_caja_redondeada(
            rect_panel_preamp,
            10,
            COLOR_PANEL_TARJETA,
            255,
            Some(COLOR_BORDE_SUTIL),
        );
        self.lienzo.dibujar_texto(
            "PREAMP",
            RectanguloUi {
                x: rect_panel_preamp.x,
                y: rect_panel_preamp.y + 8,
                ancho: rect_panel_preamp.ancho,
                alto: 20,
            },
            COLOR_AMBAR_SUNLIGHT,
            self.lienzo.fuente_subtitulo,
            DT_CENTER,
        );

        let pista_fader = RectanguloUi {
            x: rect_panel_preamp.x + 44,
            y: rect_panel_preamp.y + 36,
            ancho: 16,
            alto: (alto_inferior - 78).max(60),
        };
        self.rect_fader_preamp = RectanguloUi {
            x: rect_panel_preamp.x + 20,
            y: pista_fader.y,
            ancho: 64,
            alto: pista_fader.alto,
        };
        self.lienzo
            .dibujar_caja_redondeada(pista_fader, 6, 0x080B12, 255, Some(COLOR_BORDE_SUTIL));

        let prop_preamp = ((24.0 - estado.ganancia_preamp_db) / 48.0).clamp(0.0, 1.0) as f32;
        let y_perilla_pre = pista_fader.y + ((pista_fader.alto as f32) * prop_preamp) as i32;
        let y_centro_pre = pista_fader.y + pista_fader.alto / 2;
        let y_fill_min = y_perilla_pre.min(y_centro_pre);
        let y_fill_max = y_perilla_pre.max(y_centro_pre);
        self.lienzo.llenar_rectangulo(
            RectanguloUi {
                x: pista_fader.x + 3,
                y: y_fill_min,
                ancho: pista_fader.ancho - 6,
                alto: (y_fill_max - y_fill_min).max(2),
            },
            COLOR_AMBAR_SUNLIGHT,
            220,
        );
        self.lienzo.dibujar_caja_redondeada(
            RectanguloUi {
                x: rect_panel_preamp.x + 28,
                y: y_perilla_pre - 7,
                ancho: 48,
                alto: 14,
            },
            4,
            COLOR_AMBAR_SUNLIGHT,
            255,
            None,
        );

        let txt_preamp = format!("{:+.1} dB", estado.ganancia_preamp_db);
        self.lienzo.dibujar_texto(
            &txt_preamp,
            RectanguloUi {
                x: rect_panel_preamp.x + 4,
                y: rect_panel_preamp.y + alto_inferior - 32,
                ancho: rect_panel_preamp.ancho - 8,
                alto: 22,
            },
            COLOR_TEXTO_BLANCO,
            self.lienzo.fuente_mono,
            DT_CENTER,
        );
        self.zonas_clic.push(ZonaInteractiva {
            rect: self.rect_fader_preamp,
            identificador: AccionZona::IniciarArrastre(
                ControlArrastrado::DeslizadorPreamp,
                estado.ganancia_preamp_db,
            ),
        });

        // B. Panel Derecho: 2 Grandes Perillas de Tono (`DSPToneRoundKnobLayout`: Graves y Agudos)
        let ancho_panel_tono = 320;
        let rect_panel_tono = RectanguloUi {
            x: area.x + area.ancho - ancho_panel_tono,
            y: y_inferior,
            ancho: ancho_panel_tono,
            alto: alto_inferior,
        };
        self.lienzo.dibujar_caja_redondeada(
            rect_panel_tono,
            10,
            COLOR_PANEL_TARJETA,
            255,
            Some(COLOR_BORDE_SUTIL),
        );
        self.lienzo.dibujar_texto(
            "TONO MAESTRO (GRAVES & AGUDOS)",
            RectanguloUi {
                x: rect_panel_tono.x + 12,
                y: rect_panel_tono.y + 8,
                ancho: rect_panel_tono.ancho - 24,
                alto: 20,
            },
            COLOR_TEXTO_GRIS,
            self.lienzo.fuente_subtitulo,
            DT_CENTER,
        );

        let radio_perilla_tono = ((alto_inferior - 92) as f32 * 0.38).clamp(32.0, 52.0);
        let cy_tono = (rect_panel_tono.y + 34) as f32 + radio_perilla_tono + 14.0;

        // Perilla Graves (Bass)
        let cx_graves = (rect_panel_tono.x + 84) as f32;
        self.dibujar_perilla_rotatoria_poweramp(
            cx_graves,
            cy_tono,
            radio_perilla_tono,
            estado.tono.graves_ganancia_db,
            -15.0,
            15.0,
            "GRAVES (BASS)",
            &format!(
                "{:+.1} dB ({:.0}Hz)",
                estado.tono.graves_ganancia_db, estado.tono.graves_frecuencia_hz
            ),
            COLOR_AMBAR_SUNLIGHT,
            ControlArrastrado::PerillaGraves,
        );

        // Perilla Agudos (Treble)
        let cx_agudos = (rect_panel_tono.x + 236) as f32;
        self.dibujar_perilla_rotatoria_poweramp(
            cx_agudos,
            cy_tono,
            radio_perilla_tono,
            estado.tono.agudos_ganancia_db,
            -15.0,
            15.0,
            "AGUDOS (TREBLE)",
            &format!(
                "{:+.1} dB ({:.0}kHz)",
                estado.tono.agudos_ganancia_db,
                estado.tono.agudos_frecuencia_hz / 1000.0
            ),
            COLOR_CIAN_ELECTRICO,
            ControlArrastrado::PerillaAgudos,
        );

        // C. Panel Central: Lista de Bandas Parametricas (`EqKnobsList`) + Presets Rapidos
        let rect_panel_bandas = RectanguloUi {
            x: rect_panel_preamp.x + rect_panel_preamp.ancho + 12,
            y: y_inferior,
            ancho: area.ancho - rect_panel_preamp.ancho - ancho_panel_tono - 24,
            alto: alto_inferior,
        };
        self.lienzo.dibujar_caja_redondeada(
            rect_panel_bandas,
            10,
            COLOR_PANEL_TARJETA,
            255,
            Some(COLOR_BORDE_SUTIL),
        );

        // Cabecera del panel de bandas con botones de Presets rapidos
        self.lienzo.dibujar_texto(
            "BANDAS PARAMETRICAS (CLIC NODO = MOVER | RUEDA = FACTOR Q | CLIC DER = TIPO FILTRO)",
            RectanguloUi {
                x: rect_panel_bandas.x + 14,
                y: rect_panel_bandas.y + 8,
                ancho: 540,
                alto: 20,
            },
            COLOR_TEXTO_GRIS,
            self.lienzo.fuente_mono_pequena,
            DT_LEFT,
        );

        let presets_rapidos = ["JBL 720BT", "BAJOS +", "VOCAL", "ROCK"];
        let mut px_preset = rect_panel_bandas.x + rect_panel_bandas.ancho - 328;
        for (idx_p, nom_p) in presets_rapidos.iter().enumerate() {
            let r_p = RectanguloUi {
                x: px_preset,
                y: rect_panel_bandas.y + 6,
                ancho: 76,
                alto: 22,
            };
            self.lienzo.dibujar_caja_redondeada(
                r_p,
                5,
                COLOR_PANEL_ELEVADO,
                255,
                Some(COLOR_BORDE_SUTIL),
            );
            self.lienzo.dibujar_texto(
                nom_p,
                r_p,
                COLOR_AMBAR_SUNLIGHT,
                self.lienzo.fuente_mono_pequena,
                DT_CENTER,
            );
            self.zonas_clic.push(ZonaInteractiva {
                rect: r_p,
                identificador: AccionZona::AplicarPresetRapido(idx_p),
            });
            px_preset += 80;
        }

        // Tarjetas de cada banda parametrica
        let num_bandas = estado.bandas.len().max(1);
        let max_visibles = num_bandas.min(12);
        let margen_x = 10;
        let ancho_disp = rect_panel_bandas.ancho - 20;
        let ancho_tarjeta = ((ancho_disp - (margen_x * (max_visibles as i32 - 1)))
            / (max_visibles as i32))
            .clamp(56, 110);

        for idx in 0..max_visibles {
            let b = &estado.bandas[idx];
            let tx = rect_panel_bandas.x + 10 + (idx as i32) * (ancho_tarjeta + margen_x);
            let ty = rect_panel_bandas.y + 34;
            let th = rect_panel_bandas.alto - 44;
            let r_card = RectanguloUi {
                x: tx,
                y: ty,
                ancho: ancho_tarjeta,
                alto: th,
            };

            let color_b = COLORES_BANDAS_POWERAMP[idx % COLORES_BANDAS_POWERAMP.len()];
            let es_sel = idx == self.banda_seleccionada;
            let borde_card = if es_sel { color_b } else { COLOR_BORDE_SUTIL };
            let fondo_card = if es_sel { 0x162033 } else { 0x0C1018 };

            self.lienzo
                .dibujar_caja_redondeada(r_card, 8, fondo_card, 255, Some(borde_card));

            // Boton superior de tipo de filtro (PK / LS / HS / BP / LP / HP)
            let codigo_corto = match b.tipo {
                TipoFiltro::PicoParametrico => "PK",
                TipoFiltro::EstanteGraves => "LS",
                TipoFiltro::EstanteAgudos => "HS",
                TipoFiltro::PasoBanda => "BP",
                TipoFiltro::PasoBajo => "LP",
                TipoFiltro::PasoAlto => "HP",
            };
            let r_tipo = RectanguloUi {
                x: tx + 4,
                y: ty + 4,
                ancho: ancho_tarjeta - 8,
                alto: 20,
            };
            self.lienzo.dibujar_caja_redondeada(
                r_tipo,
                4,
                if b.activo { color_b } else { COLOR_PANEL_ELEVADO },
                255,
                None,
            );
            let txt_cabecera_b = format!("B{} {}", idx + 1, codigo_corto);
            self.lienzo.dibujar_texto(
                &txt_cabecera_b,
                r_tipo,
                if b.activo { 0x07090E } else { COLOR_TEXTO_GRIS },
                self.lienzo.fuente_mono_pequena,
                DT_CENTER,
            );
            self.zonas_clic.push(ZonaInteractiva {
                rect: r_tipo,
                identificador: AccionZona::CiclarTipoFiltroBanda(idx),
            });

            // Mini barra vertical indicadora de ganancia de la banda
            let alto_barra_b = (th - 82).max(28);
            let y_barra_b = ty + 30;
            let x_barra_b = tx + ancho_tarjeta / 2 - 4;
            self.lienzo.llenar_rectangulo(
                RectanguloUi {
                    x: x_barra_b,
                    y: y_barra_b,
                    ancho: 8,
                    alto: alto_barra_b,
                },
                0x07090E,
                255,
            );
            let prop_b = ((18.0 - b.ganancia_db.clamp(-18.0, 18.0)) / 36.0) as f32;
            let py_b = y_barra_b + ((alto_barra_b as f32) * prop_b) as i32;
            let py_cero_b = y_barra_b + alto_barra_b / 2;
            self.lienzo.llenar_rectangulo(
                RectanguloUi {
                    x: x_barra_b + 1,
                    y: py_b.min(py_cero_b),
                    ancho: 6,
                    alto: (py_b - py_cero_b).abs().max(2),
                },
                color_b,
                230,
            );

            // Lecturas de Frecuencia, Ganancia y Factor Q
            let txt_frec = if b.frecuencia_hz >= 1000.0 {
                format!("{:.1}k", b.frecuencia_hz / 1000.0)
            } else {
                format!("{:.0}Hz", b.frecuencia_hz)
            };
            let txt_gan = format!("{:+.1}dB", b.ganancia_db);
            let txt_q = format!("Q:{:.2}", b.factor_q);

            self.lienzo.dibujar_texto(
                &txt_gan,
                RectanguloUi {
                    x: tx + 2,
                    y: ty + th - 48,
                    ancho: ancho_tarjeta - 4,
                    alto: 15,
                },
                COLOR_TEXTO_BLANCO,
                self.lienzo.fuente_mono_pequena,
                DT_CENTER,
            );
            self.lienzo.dibujar_texto(
                &txt_frec,
                RectanguloUi {
                    x: tx + 2,
                    y: ty + th - 33,
                    ancho: ancho_tarjeta - 4,
                    alto: 15,
                },
                color_b,
                self.lienzo.fuente_mono_pequena,
                DT_CENTER,
            );
            self.lienzo.dibujar_texto(
                &txt_q,
                RectanguloUi {
                    x: tx + 2,
                    y: ty + th - 18,
                    ancho: ancho_tarjeta - 4,
                    alto: 15,
                },
                COLOR_TEXTO_ATENUADO,
                self.lienzo.fuente_mono_pequena,
                DT_CENTER,
            );
            self.zonas_clic.push(ZonaInteractiva {
                rect: RectanguloUi {
                    x: tx + 2,
                    y: ty + th - 18,
                    ancho: ancho_tarjeta - 4,
                    alto: 15,
                },
                identificador: AccionZona::AlternarBandaActiva(idx),
            });

            self.zonas_clic.push(ZonaInteractiva {
                rect: RectanguloUi {
                    x: tx,
                    y: ty + 26,
                    ancho: ancho_tarjeta,
                    alto: th - 46,
                },
                identificador: AccionZona::SeleccionarBanda(idx),
            });
        }
    }

    // ========================================================================
    // ESCENA 1: VOLUMEN, ESPACIO Y COMPRESOR DINAMICO
    // (`merge_eq_vol_tab.xml` + `peq_compr.xml`)
    // ========================================================================

    unsafe fn dibujar_escena_espacio_y_compresor(
        &mut self,
        area: RectanguloUi,
        estado: &EstadoCompletoDsp,
        reduccion_comp_db: f32,
        pico_salida_db: f32,
    ) {
        let mitad_w = (area.ancho - 16) / 2;

        // 1. Panel Izquierdo: Espacio & Balance (`merge_eq_vol_tab.xml`)
        let rect_espacio = RectanguloUi {
            x: area.x,
            y: area.y,
            ancho: mitad_w,
            alto: area.alto,
        };
        self.lienzo.dibujar_caja_redondeada(
            rect_espacio,
            12,
            COLOR_PANEL_TARJETA,
            255,
            Some(COLOR_BORDE_SUTIL),
        );

        self.lienzo.dibujar_texto(
            "PROCESADOR DE ESPACIO, BALANCE Y TIEMPO (MID/SIDE)",
            RectanguloUi {
                x: rect_espacio.x + 20,
                y: rect_espacio.y + 16,
                ancho: rect_espacio.ancho - 40,
                alto: 24,
            },
            COLOR_AMBAR_SUNLIGHT,
            self.lienzo.fuente_titulo,
            DT_LEFT,
        );

        // Botones de Modo Mono y Centrar Balance
        let r_btn_mono = RectanguloUi {
            x: rect_espacio.x + 24,
            y: rect_espacio.y + 56,
            ancho: 170,
            alto: 36,
        };
        self.lienzo.dibujar_caja_redondeada(
            r_btn_mono,
            8,
            if estado.espacio.modo_mono {
                COLOR_AMBAR_SUNLIGHT
            } else {
                COLOR_PANEL_ELEVADO
            },
            255,
            Some(COLOR_BORDE_SUTIL),
        );
        self.lienzo.dibujar_texto(
            if estado.espacio.modo_mono {
                "MEZCLA MONO: ACTIVA"
            } else {
                "MEZCLA MONO: INACTIVA"
            },
            r_btn_mono,
            if estado.espacio.modo_mono {
                0x07090E
            } else {
                COLOR_TEXTO_BLANCO
            },
            self.lienzo.fuente_mono_pequena,
            DT_CENTER,
        );
        self.zonas_clic.push(ZonaInteractiva {
            rect: r_btn_mono,
            identificador: AccionZona::AlternarModoMono,
        });

        let r_btn_centro = RectanguloUi {
            x: rect_espacio.x + 210,
            y: rect_espacio.y + 56,
            ancho: 170,
            alto: 36,
        };
        self.lienzo.dibujar_caja_redondeada(
            r_btn_centro,
            8,
            COLOR_PANEL_ELEVADO,
            255,
            Some(COLOR_BORDE_SUTIL),
        );
        self.lienzo.dibujar_texto(
            "CENTRAR BALANCE (0.0)",
            r_btn_centro,
            COLOR_CIAN_ELECTRICO,
            self.lienzo.fuente_mono_pequena,
            DT_CENTER,
        );
        self.zonas_clic.push(ZonaInteractiva {
            rect: r_btn_centro,
            identificador: AccionZona::CentrarBalance,
        });

        // 2 Grandes Perillas de Espacio (Balance L/R, Expansion Estereo)
        let cy_esp = (rect_espacio.y + 230) as f32;
        let paso_x_esp = rect_espacio.ancho / 2;

        self.dibujar_perilla_rotatoria_poweramp(
            (rect_espacio.x + paso_x_esp / 2) as f32,
            cy_esp,
            70.0,
            estado.espacio.balance,
            -1.0,
            1.0,
            "BALANCE L / R",
            &format!("{:+.2}", estado.espacio.balance),
            COLOR_AMBAR_SUNLIGHT,
            ControlArrastrado::PerillaBalance,
        );

        self.dibujar_perilla_rotatoria_poweramp(
            (rect_espacio.x + paso_x_esp + paso_x_esp / 2) as f32,
            cy_esp,
            70.0,
            estado.espacio.expansion_estereo,
            0.0,
            1.0,
            "EXPANSION ESTEREO",
            &format!("{:.0} %", estado.espacio.expansion_estereo * 100.0),
            COLOR_CIAN_ELECTRICO,
            ControlArrastrado::PerillaExpansionEstereo,
        );

        // Explicacion tecnica de la matriz Mid/Side en la parte inferior
        let r_info_ms = RectanguloUi {
            x: rect_espacio.x + 24,
            y: rect_espacio.y + rect_espacio.alto - 140,
            ancho: rect_espacio.ancho - 48,
            alto: 112,
        };
        self.lienzo.dibujar_caja_redondeada(
            r_info_ms,
            8,
            COLOR_PANEL_SUPERIOR,
            255,
            Some(COLOR_BORDE_SUTIL),
        );
        self.lienzo.dibujar_texto(
            "MATRIZ ESTEREO MID/SIDE DE PRECISION 64-BIT",
            RectanguloUi {
                x: r_info_ms.x + 16,
                y: r_info_ms.y + 14,
                ancho: r_info_ms.ancho - 32,
                alto: 22,
            },
            COLOR_CIAN_ELECTRICO,
            self.lienzo.fuente_subtitulo,
            DT_LEFT,
        );
        self.lienzo.dibujar_texto(
            "Separa el canal central (Mid = (L+R)*0.5) del diferencial (Side = (L-R)*0.5) para expandir la escena acustica sin perdida de fase.",
            RectanguloUi {
                x: r_info_ms.x + 16,
                y: r_info_ms.y + 42,
                ancho: r_info_ms.ancho - 32,
                alto: 24,
            },
            COLOR_TEXTO_GRIS,
            self.lienzo.fuente_normal,
            DT_LEFT,
        );
        self.lienzo.dibujar_texto(
            "Arrastre cualquier perilla verticalmente con el raton para ajustar en tiempo real.",
            RectanguloUi {
                x: r_info_ms.x + 16,
                y: r_info_ms.y + 70,
                ancho: r_info_ms.ancho - 32,
                alto: 22,
            },
            COLOR_AMBAR_SUNLIGHT,
            self.lienzo.fuente_mono_pequena,
            DT_LEFT,
        );

        // 2. Panel Derecho: Compresor Dinamico (`peq_compr.xml`)
        let rect_comp = RectanguloUi {
            x: area.x + mitad_w + 16,
            y: area.y,
            ancho: mitad_w,
            alto: area.alto,
        };
        self.lienzo.dibujar_caja_redondeada(
            rect_comp,
            12,
            COLOR_PANEL_TARJETA,
            255,
            Some(COLOR_BORDE_SUTIL),
        );

        self.lienzo.dibujar_texto(
            "COMPRESOR DINAMICO ESTEREO (SIDECHAIN)",
            RectanguloUi {
                x: rect_comp.x + 20,
                y: rect_comp.y + 16,
                ancho: rect_comp.ancho - 200,
                alto: 24,
            },
            COLOR_CHERRY_NEON,
            self.lienzo.fuente_titulo,
            DT_LEFT,
        );

        let r_btn_comp = RectanguloUi {
            x: rect_comp.x + rect_comp.ancho - 165,
            y: rect_comp.y + 14,
            ancho: 145,
            alto: 34,
        };
        self.lienzo.dibujar_caja_redondeada(
            r_btn_comp,
            7,
            if estado.compresor.activo {
                COLOR_CHERRY_NEON
            } else {
                COLOR_PANEL_ELEVADO
            },
            255,
            Some(COLOR_BORDE_SUTIL),
        );
        self.lienzo.dibujar_texto(
            if estado.compresor.activo {
                "COMPRESOR: ON"
            } else {
                "COMPRESOR: OFF"
            },
            r_btn_comp,
            if estado.compresor.activo {
                0x07090E
            } else {
                COLOR_TEXTO_BLANCO
            },
            self.lienzo.fuente_mono_pequena,
            DT_CENTER,
        );
        self.zonas_clic.push(ZonaInteractiva {
            rect: r_btn_comp,
            identificador: AccionZona::AlternarCompresorActivo,
        });

        // Vumetro de Reduccion de Ganancia (GR dB)
        let r_medidor_gr = RectanguloUi {
            x: rect_comp.x + 24,
            y: rect_comp.y + 58,
            ancho: rect_comp.ancho - 48,
            alto: 32,
        };
        self.lienzo.dibujar_caja_redondeada(
            r_medidor_gr,
            6,
            COLOR_PANEL_SUPERIOR,
            255,
            Some(COLOR_BORDE_SUTIL),
        );
        let prop_gr = (reduccion_comp_db / 24.0).clamp(0.0, 1.0);
        let ancho_gr = ((r_medidor_gr.ancho - 8) as f32 * prop_gr) as i32;
        if ancho_gr > 2 {
            self.lienzo.llenar_rectangulo(
                RectanguloUi {
                    x: r_medidor_gr.x + 4,
                    y: r_medidor_gr.y + 4,
                    ancho: ancho_gr,
                    alto: r_medidor_gr.alto - 8,
                },
                COLOR_CHERRY_NEON,
                190,
            );
        }
        let txt_gr = format!(
            "REDUCCION DE GANANCIA (GR): -{:.2} dB   |   PICO SALIDA: {:.1} dBFS",
            reduccion_comp_db, pico_salida_db
        );
        self.lienzo.dibujar_texto(
            &txt_gr,
            r_medidor_gr,
            COLOR_TEXTO_BLANCO,
            self.lienzo.fuente_mono_pequena,
            DT_CENTER,
        );

        // 2 Filas de 3 Perillas para el Compresor
        let paso_cx = rect_comp.ancho / 3;
        let cy_fila1 = (rect_comp.y + 185) as f32;
        let cy_fila2 = (rect_comp.y + 375) as f32;

        self.dibujar_perilla_rotatoria_poweramp(
            (rect_comp.x + paso_cx / 2) as f32,
            cy_fila1,
            52.0,
            estado.compresor.limite_db,
            -60.0,
            0.0,
            "UMBRAL (THRESHOLD)",
            &format!("{:.1} dB", estado.compresor.limite_db),
            COLOR_AMBAR_SUNLIGHT,
            ControlArrastrado::PerillaCompUmbral,
        );
        self.dibujar_perilla_rotatoria_poweramp(
            (rect_comp.x + paso_cx + paso_cx / 2) as f32,
            cy_fila1,
            52.0,
            estado.compresor.proporcion,
            1.0,
            20.0,
            "PROPORCION (RATIO)",
            &format!("{:.1} : 1", estado.compresor.proporcion),
            COLOR_CHERRY_NEON,
            ControlArrastrado::PerillaCompProporcion,
        );
        self.dibujar_perilla_rotatoria_poweramp(
            (rect_comp.x + 2 * paso_cx + paso_cx / 2) as f32,
            cy_fila1,
            52.0,
            estado.compresor.ataque_ms,
            0.1,
            200.0,
            "ATAQUE (ATTACK)",
            &format!("{:.1} ms", estado.compresor.ataque_ms),
            COLOR_CIAN_ELECTRICO,
            ControlArrastrado::PerillaCompAtaque,
        );

        self.dibujar_perilla_rotatoria_poweramp(
            (rect_comp.x + paso_cx / 2) as f32,
            cy_fila2,
            52.0,
            estado.compresor.lanzamiento_ms,
            10.0,
            1000.0,
            "LIBERACION (RELEASE)",
            &format!("{:.0} ms", estado.compresor.lanzamiento_ms),
            COLOR_CIAN_ELECTRICO,
            ControlArrastrado::PerillaCompLiberacion,
        );
        self.dibujar_perilla_rotatoria_poweramp(
            (rect_comp.x + paso_cx + paso_cx / 2) as f32,
            cy_fila2,
            52.0,
            estado.compresor.frecuencia_sidechain_hz,
            20.0,
            2000.0,
            "FILTRO SIDECHAIN",
            &format!("{:.0} Hz", estado.compresor.frecuencia_sidechain_hz),
            COLOR_VERDE_ESMERALDA,
            ControlArrastrado::PerillaCompSidechain,
        );
        self.dibujar_perilla_rotatoria_poweramp(
            (rect_comp.x + 2 * paso_cx + paso_cx / 2) as f32,
            cy_fila2,
            52.0,
            estado.compresor.ganancia_maquillaje_db,
            -12.0,
            24.0,
            "MAQUILLAJE (MAKEUP)",
            &format!("{:+.1} dB", estado.compresor.ganancia_maquillaje_db),
            COLOR_AMBAR_SUNLIGHT,
            ControlArrastrado::PerillaCompMaquillaje,
        );
    }

    // ========================================================================
    // ESCENA 2: REVERBERACION ESTEREO (`merge_eq_reverb_tab.xml`)
    // ========================================================================

    unsafe fn dibujar_escena_reverberacion(
        &mut self,
        area: RectanguloUi,
        estado: &EstadoCompletoDsp,
    ) {
        self.lienzo.dibujar_caja_redondeada(
            area,
            12,
            COLOR_PANEL_TARJETA,
            255,
            Some(COLOR_BORDE_SUTIL),
        );

        self.lienzo.dibujar_texto(
            "REVERBERACION ESTEREO ALGORITMICA SCHROEDER-MOORER / FREEVERB",
            RectanguloUi {
                x: area.x + 24,
                y: area.y + 18,
                ancho: 650,
                alto: 26,
            },
            COLOR_CIAN_ELECTRICO,
            self.lienzo.fuente_titulo,
            DT_LEFT,
        );

        // Interruptor Reverb Activa
        let r_btn_rev = RectanguloUi {
            x: area.x + 24,
            y: area.y + 58,
            ancho: 210,
            alto: 38,
        };
        self.lienzo.dibujar_caja_redondeada(
            r_btn_rev,
            8,
            if estado.reverberacion.activo {
                COLOR_CIAN_ELECTRICO
            } else {
                COLOR_PANEL_ELEVADO
            },
            255,
            Some(COLOR_BORDE_SUTIL),
        );
        self.lienzo.dibujar_texto(
            if estado.reverberacion.activo {
                "REVERBERACION: ACTIVA"
            } else {
                "REVERBERACION: INACTIVA"
            },
            r_btn_rev,
            if estado.reverberacion.activo {
                0x07090E
            } else {
                COLOR_TEXTO_BLANCO
            },
            self.lienzo.fuente_mono,
            DT_CENTER,
        );
        self.zonas_clic.push(ZonaInteractiva {
            rect: r_btn_rev,
            identificador: AccionZona::AlternarReverbActiva,
        });

        // Presets de Sala (`ReverbPresetPopupListLayout`)
        let presets_rev = [
            "ESTUDIO SECO",
            "SALA ACUSTICA",
            "AUDITORIO",
            "CATEDRAL PROFUNDA",
            "PLACA VINTAGE",
        ];
        let mut rx_p = area.x + 254;
        for (idx_r, nom_r) in presets_rev.iter().enumerate() {
            let r_p = RectanguloUi {
                x: rx_p,
                y: area.y + 58,
                ancho: 155,
                alto: 38,
            };
            self.lienzo.dibujar_caja_redondeada(
                r_p,
                8,
                COLOR_PANEL_SUPERIOR,
                255,
                Some(COLOR_BORDE_SUTIL),
            );
            self.lienzo.dibujar_texto(
                nom_r,
                r_p,
                COLOR_AMBAR_SUNLIGHT,
                self.lienzo.fuente_mono_pequena,
                DT_CENTER,
            );
            self.zonas_clic.push(ZonaInteractiva {
                rect: r_p,
                identificador: AccionZona::AplicarPresetReverb(idx_r),
            });
            rx_p += 165;
        }

        // 5 Grandes Perillas de Reverberacion
        let paso_x = area.ancho / 5;
        let cy_rev = (area.y + 270) as f32;

        self.dibujar_perilla_rotatoria_poweramp(
            (area.x + paso_x / 2) as f32,
            cy_rev,
            68.0,
            estado.reverberacion.tamano,
            0.0,
            1.0,
            "TAMANO DE SALA",
            &format!("{:.0} %", estado.reverberacion.tamano * 100.0),
            COLOR_CIAN_ELECTRICO,
            ControlArrastrado::PerillaRevTamano,
        );

        self.dibujar_perilla_rotatoria_poweramp(
            (area.x + paso_x + paso_x / 2) as f32,
            cy_rev,
            68.0,
            estado.reverberacion.preretardo_ms,
            0.0,
            100.0,
            "PRE-RETARDO",
            &format!("{:.1} ms", estado.reverberacion.preretardo_ms),
            COLOR_AMBAR_SUNLIGHT,
            ControlArrastrado::PerillaRevPreretardo,
        );

        self.dibujar_perilla_rotatoria_poweramp(
            (area.x + 2 * paso_x + paso_x / 2) as f32,
            cy_rev,
            68.0,
            estado.reverberacion.amortiguacion,
            0.0,
            1.0,
            "AMORTIGUACION HF",
            &format!("{:.0} %", estado.reverberacion.amortiguacion * 100.0),
            COLOR_VERDE_ESMERALDA,
            ControlArrastrado::PerillaRevAmortiguacion,
        );

        self.dibujar_perilla_rotatoria_poweramp(
            (area.x + 3 * paso_x + paso_x / 2) as f32,
            cy_rev,
            68.0,
            estado.reverberacion.filtro_tonal,
            0.0,
            1.0,
            "FILTRO TONAL",
            &format!("{:.0} %", estado.reverberacion.filtro_tonal * 100.0),
            COLOR_AMBAR_SUNLIGHT,
            ControlArrastrado::PerillaRevFiltroTonal,
        );

        self.dibujar_perilla_rotatoria_poweramp(
            (area.x + 4 * paso_x + paso_x / 2) as f32,
            cy_rev,
            68.0,
            estado.reverberacion.mezclado,
            0.0,
            1.0,
            "MEZCLADO (WET)",
            &format!("{:.0} %", estado.reverberacion.mezclado * 100.0),
            COLOR_CHERRY_NEON,
            ControlArrastrado::PerillaRevMezclado,
        );
    }

    // ========================================================================
    // ESCENA 3: BUSCADOR AUTOEQ (5,467 AUDIFONOS) & ENRUTAMIENTO WASAPI / APO COM
    // ========================================================================

    unsafe fn dibujar_escena_autoeq_y_dispositivos(
        &mut self,
        area: RectanguloUi,
        _estado: &EstadoCompletoDsp,
        enrutamiento: &EnrutamientoWasapiEstado,
    ) {
        let ancho_izq = (area.ancho as f32 * 0.56) as i32;
        let ancho_der = area.ancho - ancho_izq - 16;

        // 1. Panel Izquierdo: Base de Datos `auto_eq.db` (5,467 Perfiles de Audifonos)
        let rect_autoeq = RectanguloUi {
            x: area.x,
            y: area.y,
            ancho: ancho_izq,
            alto: area.alto,
        };
        self.lienzo.dibujar_caja_redondeada(
            rect_autoeq,
            12,
            COLOR_PANEL_TARJETA,
            255,
            Some(COLOR_BORDE_SUTIL),
        );

        self.lienzo.dibujar_texto(
            "BASE DE DATOS AUTOEQ EMBEBIDA (5,467 AUDIFONOS EXTRAIDOS DEL APK)",
            RectanguloUi {
                x: rect_autoeq.x + 20,
                y: rect_autoeq.y + 14,
                ancho: rect_autoeq.ancho - 40,
                alto: 24,
            },
            COLOR_AMBAR_SUNLIGHT,
            self.lienzo.fuente_subtitulo,
            DT_LEFT,
        );

        // Caja de texto del buscador en tiempo real
        let rect_buscador = RectanguloUi {
            x: rect_autoeq.x + 20,
            y: rect_autoeq.y + 44,
            ancho: rect_autoeq.ancho - 40,
            alto: 38,
        };
        let borde_buscador = if self.buscador_autoeq_enfocado {
            COLOR_AMBAR_SUNLIGHT
        } else {
            COLOR_CIAN_ELECTRICO
        };
        self.lienzo.dibujar_caja_redondeada(
            rect_buscador,
            8,
            0x07090E,
            255,
            Some(borde_buscador),
        );
        let texto_caja = format!(
            "BUSCAR AUDIFONOS (TECLEE MARCA O MODELO):  {}_",
            self.consulta_autoeq
        );
        self.lienzo.dibujar_texto(
            &texto_caja,
            RectanguloUi {
                x: rect_buscador.x + 14,
                y: rect_buscador.y,
                ancho: rect_buscador.ancho - 28,
                alto: rect_buscador.alto,
            },
            COLOR_TEXTO_BLANCO,
            self.lienzo.fuente_mono,
            DT_LEFT,
        );
        self.zonas_clic.push(ZonaInteractiva {
            rect: rect_buscador,
            identificador: AccionZona::EnfocarBuscadorAutoEq,
        });

        // Cuadricula de resultados de AutoEQ (2 columnas)
        let col_w = (rect_autoeq.ancho - 52) / 2;
        let alto_item = 36;
        let mut fila = 0;
        let mut col = 0;
        let resultados_clon = self.resultados_autoeq.clone();

        for item in resultados_clon.iter().take(22) {
            let ix = rect_autoeq.x + 20 + col * (col_w + 12);
            let iy = rect_autoeq.y + 94 + fila * (alto_item + 8);
            if iy + alto_item > rect_autoeq.y + rect_autoeq.alto - 12 {
                break;
            }
            let r_item = RectanguloUi {
                x: ix,
                y: iy,
                ancho: col_w,
                alto: alto_item,
            };
            self.lienzo.dibujar_caja_redondeada(
                r_item,
                6,
                COLOR_PANEL_SUPERIOR,
                255,
                Some(COLOR_BORDE_SUTIL),
            );
            let etiqueta_modelo = format!(
                "{} ({} bandas)",
                truncar_texto(&item.nombre_modelo, 28),
                item.numero_bandas
            );
            self.lienzo.dibujar_texto(
                &etiqueta_modelo,
                RectanguloUi {
                    x: r_item.x + 10,
                    y: r_item.y,
                    ancho: r_item.ancho - 20,
                    alto: r_item.alto,
                },
                COLOR_TEXTO_BLANCO,
                self.lienzo.fuente_mono_pequena,
                DT_LEFT,
            );
            self.zonas_clic.push(ZonaInteractiva {
                rect: r_item,
                identificador: AccionZona::CargarModeloAutoEq(item.nombre_modelo.clone()),
            });

            col += 1;
            if col >= 2 {
                col = 0;
                fila += 1;
            }
        }

        // 2. Panel Derecho: Dispositivos de Salida Windows WASAPI + Estado APO COM Nativo
        let rect_disp = RectanguloUi {
            x: area.x + ancho_izq + 16,
            y: area.y,
            ancho: ancho_der,
            alto: area.alto,
        };
        self.lienzo.dibujar_caja_redondeada(
            rect_disp,
            12,
            COLOR_PANEL_TARJETA,
            255,
            Some(COLOR_BORDE_SUTIL),
        );

        self.lienzo.dibujar_texto(
            "DISPOSITIVOS DE SALIDA WINDOWS (AUDIFONOS / ALTAVOCES)",
            RectanguloUi {
                x: rect_disp.x + 20,
                y: rect_disp.y + 14,
                ancho: rect_disp.ancho - 150,
                alto: 24,
            },
            COLOR_CIAN_ELECTRICO,
            self.lienzo.fuente_subtitulo,
            DT_LEFT,
        );

        let r_refrescar = RectanguloUi {
            x: rect_disp.x + rect_disp.ancho - 125,
            y: rect_disp.y + 12,
            ancho: 105,
            alto: 28,
        };
        self.lienzo.dibujar_caja_redondeada(
            r_refrescar,
            6,
            COLOR_PANEL_ELEVADO,
            255,
            Some(COLOR_BORDE_SUTIL),
        );
        self.lienzo.dibujar_texto(
            "REESCANEAR",
            r_refrescar,
            COLOR_AMBAR_SUNLIGHT,
            self.lienzo.fuente_mono_pequena,
            DT_CENTER,
        );
        self.zonas_clic.push(ZonaInteractiva {
            rect: r_refrescar,
            identificador: AccionZona::RefrescarDispositivosWasapi,
        });

        let mut dy_dev = rect_disp.y + 52;
        for disp in &enrutamiento.lista_dispositivos {
            if dy_dev + 68 > rect_disp.y + rect_disp.alto - 140 {
                break;
            }
            let es_cap = enrutamiento
                .id_dispositivo_captura
                .as_deref()
                == Some(disp.identificador.as_str());
            let es_sal = enrutamiento
                .id_dispositivo_salida_directa
                .as_deref()
                == Some(disp.identificador.as_str());

            let r_card_dev = RectanguloUi {
                x: rect_disp.x + 20,
                y: dy_dev,
                ancho: rect_disp.ancho - 40,
                alto: 62,
            };
            let color_borde_dev = if es_cap || es_sal {
                COLOR_VERDE_ESMERALDA
            } else {
                COLOR_BORDE_SUTIL
            };
            self.lienzo.dibujar_caja_redondeada(
                r_card_dev,
                8,
                COLOR_PANEL_SUPERIOR,
                255,
                Some(color_borde_dev),
            );

            let etiqueta_dev = if disp.es_predeterminado {
                format!("{} [PREDETERMINADO WINDOWS]", disp.nombre_amigable)
            } else {
                disp.nombre_amigable.clone()
            };
            self.lienzo.dibujar_texto(
                &truncar_texto(&etiqueta_dev, 52),
                RectanguloUi {
                    x: r_card_dev.x + 12,
                    y: r_card_dev.y + 6,
                    ancho: r_card_dev.ancho - 24,
                    alto: 22,
                },
                COLOR_TEXTO_BLANCO,
                self.lienzo.fuente_subtitulo,
                DT_LEFT,
            );

            // Boton Seleccionar como Captura Loopback
            let r_btn_cap = RectanguloUi {
                x: r_card_dev.x + 12,
                y: r_card_dev.y + 32,
                ancho: 215,
                alto: 24,
            };
            self.lienzo.dibujar_caja_redondeada(
                r_btn_cap,
                5,
                if es_cap {
                    COLOR_VERDE_ESMERALDA
                } else {
                    COLOR_PANEL_ELEVADO
                },
                255,
                None,
            );
            self.lienzo.dibujar_texto(
                if es_cap {
                    "CAPTURA LOOPBACK: ACTIVA"
                } else {
                    "USAR EN CAPTURA LOOPBACK"
                },
                r_btn_cap,
                if es_cap { 0x07090E } else { COLOR_TEXTO_GRIS },
                self.lienzo.fuente_mono_pequena,
                DT_CENTER,
            );
            self.zonas_clic.push(ZonaInteractiva {
                rect: r_btn_cap,
                identificador: AccionZona::SeleccionarDispositivoCaptura(
                    disp.identificador.clone(),
                    disp.nombre_amigable.clone(),
                ),
            });

            // Boton Seleccionar como Salida Directa WASAPI (`IAudioRenderClient`)
            let r_btn_sal = RectanguloUi {
                x: r_card_dev.x + 238,
                y: r_card_dev.y + 32,
                ancho: 235,
                alto: 24,
            };
            self.lienzo.dibujar_caja_redondeada(
                r_btn_sal,
                5,
                if es_sal && enrutamiento.usar_salida_directa_wasapi {
                    COLOR_CHERRY_NEON
                } else {
                    COLOR_PANEL_ELEVADO
                },
                255,
                None,
            );
            self.lienzo.dibujar_texto(
                if es_sal && enrutamiento.usar_salida_directa_wasapi {
                    "SALIDA DIRECTA WASAPI: ON"
                } else {
                    "ENVIAR SALIDA DIRECTA AQUI"
                },
                r_btn_sal,
                if es_sal && enrutamiento.usar_salida_directa_wasapi {
                    0x07090E
                } else {
                    COLOR_TEXTO_GRIS
                },
                self.lienzo.fuente_mono_pequena,
                DT_CENTER,
            );
            self.zonas_clic.push(ZonaInteractiva {
                rect: r_btn_sal,
                identificador: AccionZona::SeleccionarDispositivoSalidaDirecta(
                    disp.identificador.clone(),
                    disp.nombre_amigable.clone(),
                ),
            });

            dy_dev += 70;
        }

        // Tarjeta informativa del Objeto APO COM Soberano en Rust
        let r_apo_info = RectanguloUi {
            x: rect_disp.x + 20,
            y: rect_disp.y + rect_disp.alto - 126,
            ancho: rect_disp.ancho - 40,
            alto: 110,
        };
        self.lienzo.dibujar_caja_redondeada(
            r_apo_info,
            8,
            0x090D16,
            255,
            Some(COLOR_AMBAR_SUNLIGHT),
        );
        self.lienzo.dibujar_texto(
            "MOTOR APO COM SOBERANO EN RUST (SIN EQUALIZER APO EXTERNO)",
            RectanguloUi {
                x: r_apo_info.x + 14,
                y: r_apo_info.y + 10,
                ancho: r_apo_info.ancho - 28,
                alto: 20,
            },
            COLOR_AMBAR_SUNLIGHT,
            self.lienzo.fuente_subtitulo,
            DT_LEFT,
        );
        let txt_clsid = format!(
            "CLSID COM: {}  |  IAudioProcessingObjectRT::APOProcess",
            CLSID_APO_SOBERANO_TEXTO
        );
        self.lienzo.dibujar_texto(
            &txt_clsid,
            RectanguloUi {
                x: r_apo_info.x + 14,
                y: r_apo_info.y + 36,
                ancho: r_apo_info.ancho - 28,
                alto: 20,
            },
            COLOR_CIAN_ELECTRICO,
            self.lienzo.fuente_mono_pequena,
            DT_LEFT,
        );
        let txt_ipc = format!(
            "IPC Binario Tiempo Real: {}",
            self.ruta_ipc_binario.display()
        );
        self.lienzo.dibujar_texto(
            &txt_ipc,
            RectanguloUi {
                x: r_apo_info.x + 14,
                y: r_apo_info.y + 62,
                ancho: r_apo_info.ancho - 28,
                alto: 20,
            },
            COLOR_TEXTO_GRIS,
            self.lienzo.fuente_mono_pequena,
            DT_LEFT,
        );
    }

    // ========================================================================
    // COMPONENTE REUTILIZABLE: PERILLA ROTATORIA 3D POWERAMP (`DSPRoundKnobLayout`)
    // ========================================================================

    #[allow(clippy::too_many_arguments)]
    unsafe fn dibujar_perilla_rotatoria_poweramp(
        &mut self,
        cx: f32,
        cy: f32,
        radio: f32,
        valor: f64,
        min_val: f64,
        max_val: f64,
        titulo: &str,
        lectura_valor: &str,
        color_acento: u32,
        control_id: ControlArrastrado,
    ) {
        let proporcion = ((valor - min_val) / (max_val - min_val)).clamp(0.0, 1.0) as f32;
        // Angulos de 135 grados (abajo-izquierda) a 405 grados (abajo-derecha)
        let ang_inicio = 135.0_f32.to_radians();
        let ang_fin = 405.0_f32.to_radians();
        let ang_actual = ang_inicio + (ang_fin - ang_inicio) * proporcion;

        // Sombra exterior y cuerpo metalico oscuro de la perilla
        self.lienzo
            .dibujar_circulo_relleno(cx, cy + 3.0, radio + 2.0, 0x030508, 220);
        self.lienzo
            .dibujar_circulo_relleno(cx, cy, radio, 0x131A29, 255);
        self.lienzo
            .dibujar_circulo_relleno(cx, cy, radio * 0.80, 0x0B0F18, 255);

        // Arco base gris y arco luminoso activo
        self.lienzo.dibujar_arco_perilla(
            cx,
            cy,
            radio + 6.0,
            ang_inicio,
            ang_fin,
            3.5,
            0x1E293B,
            255,
        );
        if proporcion > 0.005 {
            self.lienzo.dibujar_arco_perilla(
                cx,
                cy,
                radio + 6.0,
                ang_inicio,
                ang_actual,
                4.0,
                color_acento,
                255,
            );
        }

        // Aguja indicadora desde el centro hacia el borde
        let r_int = radio * 0.28;
        let r_ext = radio * 0.76;
        self.lienzo.dibujar_linea(
            cx + r_int * ang_actual.cos(),
            cy + r_int * ang_actual.sin(),
            cx + r_ext * ang_actual.cos(),
            cy + r_ext * ang_actual.sin(),
            3.0,
            color_acento,
            255,
        );
        self.lienzo.dibujar_circulo_relleno(
            cx + r_ext * ang_actual.cos(),
            cy + r_ext * ang_actual.sin(),
            3.5,
            COLOR_TEXTO_BLANCO,
            255,
        );

        // Etiquetas superior (titulo) e inferior (valor numerico)
        let ancho_txt = (radio as i32 * 3).max(140);
        self.lienzo.dibujar_texto(
            titulo,
            RectanguloUi {
                x: (cx as i32) - ancho_txt / 2,
                y: (cy - radio - 30.0) as i32,
                ancho: ancho_txt,
                alto: 18,
            },
            COLOR_TEXTO_GRIS,
            self.lienzo.fuente_mono_pequena,
            DT_CENTER,
        );

        self.lienzo.dibujar_texto(
            lectura_valor,
            RectanguloUi {
                x: (cx as i32) - ancho_txt / 2,
                y: (cy + radio + 12.0) as i32,
                ancho: ancho_txt,
                alto: 20,
            },
            color_acento,
            self.lienzo.fuente_mono,
            DT_CENTER,
        );

        let rect_hit = RectanguloUi {
            x: (cx - radio - 12.0) as i32,
            y: (cy - radio - 12.0) as i32,
            ancho: ((radio + 12.0) * 2.0) as i32,
            alto: ((radio + 12.0) * 2.0) as i32,
        };
        self.zonas_clic.push(ZonaInteractiva {
            rect: rect_hit,
            identificador: AccionZona::IniciarArrastre(control_id, valor),
        });
    }

    // ========================================================================
    // BARRA INFERIOR DE NAVEGACION (`merge_navbar.xml`)
    // ========================================================================

    unsafe fn dibujar_barra_inferior_navbar(
        &mut self,
        ancho_ventana: i32,
        alto_ventana: i32,
        pico_salida_db: f32,
        reduccion_comp_db: f32,
    ) {
        let y_nav = alto_ventana - 54;
        let rect_nav = RectanguloUi {
            x: 0,
            y: y_nav,
            ancho: ancho_ventana,
            alto: 54,
        };
        self.lienzo
            .llenar_rectangulo(rect_nav, COLOR_PANEL_SUPERIOR, 255);
        self.lienzo.dibujar_linea(
            0.0,
            y_nav as f32,
            ancho_ventana as f32,
            y_nav as f32,
            1.0,
            COLOR_BORDE_SUTIL,
            255,
        );

        let pestanas: [(&str, EscenaPoweramp); 4] = [
            ("1. ECUALIZADOR & TONO", EscenaPoweramp::EcualizadorYTono),
            (
                "2. ESPACIO & COMPRESOR",
                EscenaPoweramp::EspacioYCompresor,
            ),
            ("3. REVERBERACION", EscenaPoweramp::Reverberacion),
            (
                "4. AUTOEQ (5,467) & SALIDA WASAPI",
                EscenaPoweramp::AutoEqYDispositivos,
            ),
        ];

        let mut x_tab = 18;
        for (titulo, escena) in pestanas {
            let activa = self.escena_actual == escena;
            let ancho_t = if titulo.len() > 24 { 265 } else { 205 };
            let r_tab = RectanguloUi {
                x: x_tab,
                y: y_nav + 9,
                ancho: ancho_t,
                alto: 36,
            };
            let fondo_t = if activa {
                COLOR_AMBAR_SUNLIGHT
            } else {
                COLOR_PANEL_TARJETA
            };
            let color_txt = if activa {
                0x07090E
            } else {
                COLOR_TEXTO_BLANCO
            };
            self.lienzo.dibujar_caja_redondeada(
                r_tab,
                8,
                fondo_t,
                255,
                Some(if activa {
                    COLOR_AMBAR_SUNLIGHT
                } else {
                    COLOR_BORDE_SUTIL
                }),
            );
            self.lienzo.dibujar_texto(
                titulo,
                r_tab,
                color_txt,
                self.lienzo.fuente_subtitulo,
                DT_CENTER,
            );
            self.zonas_clic.push(ZonaInteractiva {
                rect: r_tab,
                identificador: AccionZona::CambiarEscena(escena),
            });
            x_tab += ancho_t + 10;
        }

        // Telemetria derecha en la barra inferior
        let texto_telemetria = format!(
            "{}   |   PICO: {:+.1} dBFS   |   GR: -{:.1} dB",
            truncar_texto(&self.mensaje_estado_barra, 44),
            pico_salida_db,
            reduccion_comp_db
        );
        self.lienzo.dibujar_texto(
            &texto_telemetria,
            RectanguloUi {
                x: x_tab + 10,
                y: y_nav,
                ancho: (ancho_ventana - x_tab - 26).max(120),
                alto: 54,
            },
            COLOR_TEXTO_GRIS,
            self.lienzo.fuente_mono_pequena,
            DT_RIGHT,
        );
    }

    // ========================================================================
    // MANEJO DE EVENTOS DE RATON Y TECLADO EN TIEMPO REAL
    // ========================================================================

    fn manejar_clic_izquierdo(&mut self, mx: i32, my: i32) {
        // 1. Si estamos en la escena del Ecualizador y el clic cae dentro del lienzo de la curva,
        // verificar primero si se toco algun nodo parametrico para arrastrarlo.
        if self.escena_actual == EscenaPoweramp::EcualizadorYTono
            && self.rect_lienzo_curva.contiene(mx, my)
        {
            if let Ok(cadena) = self.cadena_dsp.lock() {
                let estado = cadena.obtener_estado();
                let mut mejor_idx = None;
                let mut menor_dist2 = 22.0_f32 * 22.0_f32;
                for (idx, b) in estado.bandas.iter().enumerate() {
                    let nx = frecuencia_a_x(b.frecuencia_hz, self.rect_lienzo_curva) as f32;
                    let ny = ganancia_a_y(b.ganancia_db, self.rect_lienzo_curva) as f32;
                    let dx = (mx as f32) - nx;
                    let dy = (my as f32) - ny;
                    let d2 = dx * dx + dy * dy;
                    if d2 <= menor_dist2 {
                        menor_dist2 = d2;
                        mejor_idx = Some(idx);
                    }
                }
                if let Some(idx_encontrado) = mejor_idx {
                    self.banda_seleccionada = idx_encontrado;
                    self.control_arrastrado = ControlArrastrado::NodoBanda(idx_encontrado);
                    self.buscador_autoeq_enfocado = false;
                    return;
                }
            }
        }

        // 2. Revisar zonas interactivas registradas
        let mut accion_hallada: Option<AccionZona> = None;
        for zona in self.zonas_clic.iter().rev() {
            if zona.rect.contiene(mx, my) {
                accion_hallada = Some(zona.identificador.clone());
                break;
            }
        }

        let Some(accion) = accion_hallada else {
            self.buscador_autoeq_enfocado = false;
            return;
        };

        match accion {
            AccionZona::CambiarEscena(nueva) => {
                self.escena_actual = nueva;
                self.buscador_autoeq_enfocado = nueva == EscenaPoweramp::AutoEqYDispositivos;
            }
            AccionZona::AlternarEqActivo => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    let nuevo = !c.obtener_estado().ecualizador_activo;
                    c.establecer_ecualizador_activo(nuevo);
                    self.sincronizar_persistencia_dsp(c.obtener_estado());
                }
            }
            AccionZona::AlternarTonoActivo => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    let mut t = c.obtener_estado().tono;
                    t.tono_activo = !t.tono_activo;
                    let _ = c.actualizar_tono(t);
                    self.sincronizar_persistencia_dsp(c.obtener_estado());
                }
            }
            AccionZona::AlternarLimitador => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    let nuevo = !c.obtener_estado().limitador_activo;
                    c.establecer_limitador_activo(nuevo);
                    self.sincronizar_persistencia_dsp(c.obtener_estado());
                }
            }
            AccionZona::AlternarEnrutadorDirectoWasapi => {
                if let Ok(mut enr) = self.enrutamiento.lock() {
                    enr.usar_salida_directa_wasapi = !enr.usar_salida_directa_wasapi;
                    enr.solicitar_reinicio_audio = true;
                    self.mensaje_estado_barra = if enr.usar_salida_directa_wasapi {
                        format!("Salida Directa WASAPI activa hacia: {}", enr.nombre_dispositivo_salida)
                    } else {
                        String::from("Modo Monitoreo Loopback + APO COM IPC activo")
                    };
                }
            }
            AccionZona::AgregarBandaParametrica => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    let nueva = BandaParametrica::nueva(0, 1000.0, 1.41, 3.0, TipoFiltro::PicoParametrico);
                    if c.agregar_banda(nueva).is_ok() {
                        self.banda_seleccionada = c.obtener_estado().bandas.len().saturating_sub(1);
                        self.sincronizar_persistencia_dsp(c.obtener_estado());
                    }
                }
            }
            AccionZona::EliminarBandaSeleccionada => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    let id_1 = self.banda_seleccionada + 1;
                    if c.eliminar_banda(id_1).is_ok() {
                        let total = c.obtener_estado().bandas.len();
                        if self.banda_seleccionada >= total && total > 0 {
                            self.banda_seleccionada = total - 1;
                        }
                        self.sincronizar_persistencia_dsp(c.obtener_estado());
                    }
                }
            }
            AccionZona::ReiniciarPlano => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    let _ = c.aplicar_estado(EstadoCompletoDsp::default());
                    self.banda_seleccionada = 0;
                    self.sincronizar_persistencia_dsp(c.obtener_estado());
                    self.mensaje_estado_barra = String::from("Respuesta plana (0.0 dB) restaurada");
                }
            }
            AccionZona::SeleccionarBanda(idx) => {
                self.banda_seleccionada = idx;
                self.control_arrastrado = ControlArrastrado::NodoBanda(idx);
            }
            AccionZona::AlternarBandaActiva(idx) => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    if let Some(b) = c.obtener_estado().bandas.get(idx).cloned() {
                        let _ = c.actualizar_parametros_banda(
                            idx + 1,
                            b.frecuencia_hz,
                            b.factor_q,
                            b.ganancia_db,
                            b.tipo,
                            !b.activo,
                        );
                        self.sincronizar_persistencia_dsp(c.obtener_estado());
                    }
                }
            }
            AccionZona::CiclarTipoFiltroBanda(idx) => {
                self.ciclar_tipo_filtro_de_banda(idx);
            }
            AccionZona::AplicarPresetRapido(codigo) => {
                self.aplicar_preset_rapido(codigo);
            }
            AccionZona::AlternarModoMono => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    let mut esp = c.obtener_estado().espacio;
                    esp.modo_mono = !esp.modo_mono;
                    c.actualizar_espacio(esp);
                    self.sincronizar_persistencia_dsp(c.obtener_estado());
                }
            }
            AccionZona::CentrarBalance => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    let mut esp = c.obtener_estado().espacio;
                    esp.balance = 0.0;
                    c.actualizar_espacio(esp);
                    self.sincronizar_persistencia_dsp(c.obtener_estado());
                }
            }
            AccionZona::AlternarCompresorActivo => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    let mut comp = c.obtener_estado().compresor;
                    comp.activo = !comp.activo;
                    c.actualizar_compresor(comp);
                    self.sincronizar_persistencia_dsp(c.obtener_estado());
                }
            }
            AccionZona::AlternarReverbActiva => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    let mut rev = c.obtener_estado().reverberacion;
                    rev.activo = !rev.activo;
                    if rev.activo && rev.mezclado < 0.05 {
                        rev.mezclado = 0.22;
                    }
                    c.actualizar_reverberacion(rev);
                    self.sincronizar_persistencia_dsp(c.obtener_estado());
                }
            }
            AccionZona::AplicarPresetReverb(indice_preset) => {
                if let Ok(mut c) = self.cadena_dsp.lock() {
                    let rev = match indice_preset {
                        0 => ParametrosReverberacion {
                            activo: true,
                            tamano: 0.25,
                            preretardo_ms: 8.0,
                            amortiguacion: 0.70,
                            filtro_tonal: 0.50,
                            mezclado: 0.15,
                        },
                        1 => ParametrosReverberacion {
                            activo: true,
                            tamano: 0.52,
                            preretardo_ms: 18.0,
                            amortiguacion: 0.50,
                            filtro_tonal: 0.55,
                            mezclado: 0.24,
                        },
                        2 => ParametrosReverberacion {
                            activo: true,
                            tamano: 0.75,
                            preretardo_ms: 32.0,
                            amortiguacion: 0.40,
                            filtro_tonal: 0.60,
                            mezclado: 0.30,
                        },
                        3 => ParametrosReverberacion {
                            activo: true,
                            tamano: 0.94,
                            preretardo_ms: 55.0,
                            amortiguacion: 0.25,
                            filtro_tonal: 0.42,
                            mezclado: 0.38,
                        },
                        _ => ParametrosReverberacion {
                            activo: true,
                            tamano: 0.65,
                            preretardo_ms: 12.0,
                            amortiguacion: 0.30,
                            filtro_tonal: 0.75,
                            mezclado: 0.26,
                        },
                    };
                    c.actualizar_reverberacion(rev);
                    self.sincronizar_persistencia_dsp(c.obtener_estado());
                }
            }
            AccionZona::EnfocarBuscadorAutoEq => {
                self.buscador_autoeq_enfocado = true;
            }
            AccionZona::CargarModeloAutoEq(modelo) => {
                let ruta_db = localizar_autoeq_db(&self.directorio_base);
                if let Ok(nuevo_estado) = cargar_preset_autoeq_por_consulta(&ruta_db, &modelo) {
                    if let Ok(mut c) = self.cadena_dsp.lock() {
                        let _ = c.aplicar_estado(nuevo_estado);
                        self.banda_seleccionada = 0;
                        self.sincronizar_persistencia_dsp(c.obtener_estado());
                        self.mensaje_estado_barra =
                            format!("Perfil AutoEQ cargado: {}", modelo);
                        self.escena_actual = EscenaPoweramp::EcualizadorYTono;
                    }
                }
            }
            AccionZona::SeleccionarDispositivoCaptura(id_dev, nombre_dev) => {
                if let Ok(mut enr) = self.enrutamiento.lock() {
                    enr.id_dispositivo_captura = Some(id_dev);
                    enr.nombre_dispositivo_captura = nombre_dev.clone();
                    enr.solicitar_reinicio_audio = true;
                    self.mensaje_estado_barra =
                        format!("Capturando Loopback desde: {}", nombre_dev);
                }
            }
            AccionZona::SeleccionarDispositivoSalidaDirecta(id_dev, nombre_dev) => {
                if let Ok(mut enr) = self.enrutamiento.lock() {
                    enr.id_dispositivo_salida_directa = Some(id_dev);
                    enr.nombre_dispositivo_salida = nombre_dev.clone();
                    enr.usar_salida_directa_wasapi = true;
                    enr.solicitar_reinicio_audio = true;
                    self.mensaje_estado_barra =
                        format!("Salida Directa WASAPI enviada a: {}", nombre_dev);
                }
            }
            AccionZona::RefrescarDispositivosWasapi => {
                if let Ok(lista) = enumerar_dispositivos_salida_windows() {
                    if let Ok(mut enr) = self.enrutamiento.lock() {
                        enr.lista_dispositivos = lista;
                        self.mensaje_estado_barra =
                            format!("{} dispositivos de salida detectados", enr.lista_dispositivos.len());
                    }
                }
            }
            AccionZona::IniciarArrastre(ctrl, valor_ini) => {
                self.control_arrastrado = ctrl;
                self.raton_inicio_y = my;
                self.valor_inicial_arrastre = valor_ini;
                if ctrl == ControlArrastrado::DeslizadorPreamp {
                    self.actualizar_arrastre_raton(mx, my);
                }
            }
        }
    }

    fn manejar_clic_derecho(&mut self, mx: i32, my: i32) {
        if self.escena_actual == EscenaPoweramp::EcualizadorYTono
            && self.rect_lienzo_curva.contiene(mx, my)
        {
            let mut encontrado: Option<usize> = None;
            if let Ok(cadena) = self.cadena_dsp.lock() {
                let estado = cadena.obtener_estado();
                for (idx, b) in estado.bandas.iter().enumerate() {
                    let nx = frecuencia_a_x(b.frecuencia_hz, self.rect_lienzo_curva) as f32;
                    let ny = ganancia_a_y(b.ganancia_db, self.rect_lienzo_curva) as f32;
                    let d2 = ((mx as f32) - nx).powi(2) + ((my as f32) - ny).powi(2);
                    if d2 <= 22.0 * 22.0 {
                        encontrado = Some(idx);
                        break;
                    }
                }
            }
            if let Some(idx) = encontrado {
                self.banda_seleccionada = idx;
                self.ciclar_tipo_filtro_de_banda(idx);
            }
        }
    }

    fn ciclar_tipo_filtro_de_banda(&mut self, idx: usize) {
        if let Ok(mut c) = self.cadena_dsp.lock() {
            if let Some(b) = c.obtener_estado().bandas.get(idx).cloned() {
                let siguiente = match b.tipo {
                    TipoFiltro::PicoParametrico => TipoFiltro::EstanteGraves,
                    TipoFiltro::EstanteGraves => TipoFiltro::EstanteAgudos,
                    TipoFiltro::EstanteAgudos => TipoFiltro::PasoBanda,
                    TipoFiltro::PasoBanda => TipoFiltro::PasoBajo,
                    TipoFiltro::PasoBajo => TipoFiltro::PasoAlto,
                    TipoFiltro::PasoAlto => TipoFiltro::PicoParametrico,
                };
                let _ = c.actualizar_parametros_banda(
                    idx + 1,
                    b.frecuencia_hz,
                    b.factor_q,
                    b.ganancia_db,
                    siguiente,
                    b.activo,
                );
                self.sincronizar_persistencia_dsp(c.obtener_estado());
            }
        }
    }

    fn aplicar_preset_rapido(&mut self, codigo: usize) {
        if let Ok(mut c) = self.cadena_dsp.lock() {
            match codigo {
                0 => {
                    let ruta_jbl = self
                        .directorio_base
                        .join("JBL Tune 720BT (passive) [28 jul 2026 8_55_44\u{202f}p.m.].json");
                    if let Ok(est) = cargar_preset_json_desde_archivo(&ruta_jbl) {
                        let _ = c.aplicar_estado(est);
                    }
                }
                1 => {
                    let mut est = EstadoCompletoDsp::default();
                    est.nombre_perfil = String::from("Bajos Extremos Poweramp");
                    est.ganancia_preamp_db = -3.5;
                    est.tono.graves_ganancia_db = 7.5;
                    est.tono.agudos_ganancia_db = 2.0;
                    if est.bandas.len() >= 3 {
                        est.bandas[0].ganancia_db = 6.5;
                        est.bandas[1].ganancia_db = 5.0;
                        est.bandas[2].ganancia_db = 2.5;
                    }
                    let _ = c.aplicar_estado(est);
                }
                2 => {
                    let mut est = EstadoCompletoDsp::default();
                    est.nombre_perfil = String::from("Claridad Vocal & Estudio");
                    est.ganancia_preamp_db = -1.5;
                    if est.bandas.len() >= 8 {
                        est.bandas[0].ganancia_db = -2.0;
                        est.bandas[4].ganancia_db = 2.5;
                        est.bandas[5].ganancia_db = 4.2;
                        est.bandas[6].ganancia_db = 3.8;
                        est.bandas[7].ganancia_db = 2.0;
                    }
                    let _ = c.aplicar_estado(est);
                }
                _ => {
                    let mut est = EstadoCompletoDsp::default();
                    est.nombre_perfil = String::from("Rock / Metal Acustico");
                    est.ganancia_preamp_db = -2.5;
                    est.tono.graves_ganancia_db = 4.0;
                    est.tono.agudos_ganancia_db = 4.5;
                    if est.bandas.len() >= 9 {
                        est.bandas[0].ganancia_db = 4.5;
                        est.bandas[1].ganancia_db = 3.5;
                        est.bandas[4].ganancia_db = -2.2;
                        est.bandas[7].ganancia_db = 3.8;
                        est.bandas[8].ganancia_db = 4.5;
                    }
                    let _ = c.aplicar_estado(est);
                }
            }
            self.sincronizar_persistencia_dsp(c.obtener_estado());
        }
    }

    fn actualizar_arrastre_raton(&mut self, mx: i32, my: i32) {
        if self.control_arrastrado == ControlArrastrado::Ninguno {
            return;
        }
        let delta_y = (self.raton_inicio_y - my) as f64;

        if let Ok(mut c) = self.cadena_dsp.lock() {
            match self.control_arrastrado {
                ControlArrastrado::NodoBanda(idx) => {
                    let nueva_frec = x_a_frecuencia(mx, self.rect_lienzo_curva);
                    let nueva_gan = y_a_ganancia(my, self.rect_lienzo_curva);
                    if let Some(b) = c.obtener_estado().bandas.get(idx).cloned() {
                        let _ = c.actualizar_parametros_banda(
                            idx + 1,
                            nueva_frec,
                            b.factor_q,
                            nueva_gan,
                            b.tipo,
                            b.activo,
                        );
                    }
                }
                ControlArrastrado::DeslizadorPreamp => {
                    let rf = self.rect_fader_preamp;
                    let prop = ((my - rf.y) as f64 / (rf.alto.max(1) as f64)).clamp(0.0, 1.0);
                    let db = 24.0 - prop * 48.0;
                    c.establecer_preamp_db(db);
                }
                ControlArrastrado::PerillaGraves => {
                    let mut t = c.obtener_estado().tono;
                    t.graves_ganancia_db =
                        (self.valor_inicial_arrastre + delta_y * 0.15).clamp(-15.0, 15.0);
                    let _ = c.actualizar_tono(t);
                }
                ControlArrastrado::PerillaAgudos => {
                    let mut t = c.obtener_estado().tono;
                    t.agudos_ganancia_db =
                        (self.valor_inicial_arrastre + delta_y * 0.15).clamp(-15.0, 15.0);
                    let _ = c.actualizar_tono(t);
                }
                ControlArrastrado::PerillaBalance => {
                    let mut esp = c.obtener_estado().espacio;
                    esp.balance = (self.valor_inicial_arrastre + delta_y * 0.01).clamp(-1.0, 1.0);
                    c.actualizar_espacio(esp);
                }
                ControlArrastrado::PerillaExpansionEstereo => {
                    let mut esp = c.obtener_estado().espacio;
                    esp.expansion_estereo =
                        (self.valor_inicial_arrastre + delta_y * 0.008).clamp(0.0, 1.0);
                    c.actualizar_espacio(esp);
                }
                ControlArrastrado::PerillaCompUmbral => {
                    let mut comp = c.obtener_estado().compresor;
                    comp.limite_db =
                        (self.valor_inicial_arrastre + delta_y * 0.35).clamp(-60.0, 0.0);
                    c.actualizar_compresor(comp);
                }
                ControlArrastrado::PerillaCompProporcion => {
                    let mut comp = c.obtener_estado().compresor;
                    comp.proporcion =
                        (self.valor_inicial_arrastre + delta_y * 0.12).clamp(1.0, 20.0);
                    c.actualizar_compresor(comp);
                }
                ControlArrastrado::PerillaCompAtaque => {
                    let mut comp = c.obtener_estado().compresor;
                    comp.ataque_ms =
                        (self.valor_inicial_arrastre + delta_y * 1.2).clamp(0.1, 200.0);
                    c.actualizar_compresor(comp);
                }
                ControlArrastrado::PerillaCompLiberacion => {
                    let mut comp = c.obtener_estado().compresor;
                    comp.lanzamiento_ms =
                        (self.valor_inicial_arrastre + delta_y * 5.0).clamp(10.0, 1000.0);
                    c.actualizar_compresor(comp);
                }
                ControlArrastrado::PerillaCompSidechain => {
                    let mut comp = c.obtener_estado().compresor;
                    comp.frecuencia_sidechain_hz =
                        (self.valor_inicial_arrastre + delta_y * 10.0).clamp(20.0, 2000.0);
                    c.actualizar_compresor(comp);
                }
                ControlArrastrado::PerillaCompMaquillaje => {
                    let mut comp = c.obtener_estado().compresor;
                    comp.ganancia_maquillaje_db =
                        (self.valor_inicial_arrastre + delta_y * 0.2).clamp(-12.0, 24.0);
                    c.actualizar_compresor(comp);
                }
                ControlArrastrado::PerillaRevTamano => {
                    let mut rev = c.obtener_estado().reverberacion;
                    rev.tamano = (self.valor_inicial_arrastre + delta_y * 0.008).clamp(0.0, 1.0);
                    c.actualizar_reverberacion(rev);
                }
                ControlArrastrado::PerillaRevPreretardo => {
                    let mut rev = c.obtener_estado().reverberacion;
                    rev.preretardo_ms =
                        (self.valor_inicial_arrastre + delta_y * 0.6).clamp(0.0, 100.0);
                    c.actualizar_reverberacion(rev);
                }
                ControlArrastrado::PerillaRevAmortiguacion => {
                    let mut rev = c.obtener_estado().reverberacion;
                    rev.amortiguacion =
                        (self.valor_inicial_arrastre + delta_y * 0.008).clamp(0.0, 1.0);
                    c.actualizar_reverberacion(rev);
                }
                ControlArrastrado::PerillaRevFiltroTonal => {
                    let mut rev = c.obtener_estado().reverberacion;
                    rev.filtro_tonal =
                        (self.valor_inicial_arrastre + delta_y * 0.008).clamp(0.0, 1.0);
                    c.actualizar_reverberacion(rev);
                }
                ControlArrastrado::PerillaRevMezclado => {
                    let mut rev = c.obtener_estado().reverberacion;
                    rev.mezclado = (self.valor_inicial_arrastre + delta_y * 0.008).clamp(0.0, 1.0);
                    c.actualizar_reverberacion(rev);
                }
                ControlArrastrado::Ninguno => {}
            }
            self.sincronizar_persistencia_dsp(c.obtener_estado());
        }
    }

    fn manejar_rueda_raton(&mut self, delta_rueda: i16) {
        if self.escena_actual != EscenaPoweramp::EcualizadorYTono {
            return;
        }
        let paso_q = if delta_rueda > 0 { 0.15 } else { -0.15 };
        if let Ok(mut c) = self.cadena_dsp.lock() {
            let idx = self.banda_seleccionada;
            if let Some(b) = c.obtener_estado().bandas.get(idx).cloned() {
                let nuevo_q = (b.factor_q + paso_q).clamp(0.15, 16.0);
                let _ = c.actualizar_parametros_banda(
                    idx + 1,
                    b.frecuencia_hz,
                    nuevo_q,
                    b.ganancia_db,
                    b.tipo,
                    b.activo,
                );
                self.sincronizar_persistencia_dsp(c.obtener_estado());
            }
        }
    }

    fn manejar_caracter_teclado(&mut self, codigo_char: u32) {
        if self.escena_actual != EscenaPoweramp::AutoEqYDispositivos
            && !self.buscador_autoeq_enfocado
        {
            return;
        }
        if codigo_char == 8 {
            // Backspace
            self.consulta_autoeq.pop();
            self.ejecutar_busqueda_autoeq();
        } else if codigo_char >= 32 && codigo_char <= 126 {
            if let Some(ch) = char::from_u32(codigo_char) {
                if self.consulta_autoeq.len() < 42 {
                    self.consulta_autoeq.push(ch);
                    self.ejecutar_busqueda_autoeq();
                }
            }
        }
    }
}

// ============================================================================
// FUNCIONES MATEMATICAS DE CONVERSION FRECUENCIA / GANANCIA <-> PIXELES
// ============================================================================

#[inline]
fn frecuencia_a_x(frecuencia_hz: f64, rc: RectanguloUi) -> i32 {
    let f = frecuencia_hz.clamp(20.0, 20000.0);
    let proporcion = (f / 20.0).log10() / 3.0; // log10(20000/20) = log10(1000) = 3.0
    rc.x + ((rc.ancho as f64) * proporcion).round() as i32
}

#[inline]
fn x_a_frecuencia(px: i32, rc: RectanguloUi) -> f64 {
    let proporcion = ((px - rc.x) as f64 / (rc.ancho.max(1) as f64)).clamp(0.0, 1.0);
    20.0 * 1000.0_f64.powf(proporcion)
}

#[inline]
fn ganancia_a_y(ganancia_db: f64, rc: RectanguloUi) -> i32 {
    let g = ganancia_db.clamp(-18.0, 18.0);
    let proporcion = (18.0 - g) / 36.0;
    rc.y + ((rc.alto as f64) * proporcion).round() as i32
}

#[inline]
fn y_a_ganancia(py: i32, rc: RectanguloUi) -> f64 {
    let proporcion = ((py - rc.y) as f64 / (rc.alto.max(1) as f64)).clamp(0.0, 1.0);
    18.0 - proporcion * 36.0
}

fn truncar_texto(texto: &str, max_chars: usize) -> String {
    if texto.chars().count() <= max_chars {
        texto.to_string()
    } else {
        let mut s: String = texto.chars().take(max_chars.saturating_sub(2)).collect();
        s.push_str("..");
        s
    }
}

fn localizar_autoeq_db(base: &Path) -> PathBuf {
    let r1 = base.join("recursos").join("auto_eq.db");
    if r1.exists() {
        return r1;
    }
    base.join("auto_eq.db")
}

// ============================================================================
// PROCEDIMIENTO DE VENTANA WIN32 (`wnd_proc_poweramp`) Y LANZADOR PRINCIPAL
// ============================================================================

unsafe extern "system" fn wnd_proc_poweramp(
    hwnd: *mut c_void,
    msg: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    let ctx_ptr = PUNTERO_CONTEXTO_GLOBAL;
    if ctx_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, w_param, l_param);
    }
    let ctx = &mut *ctx_ptr;

    match msg {
        WM_ERASEBKGND => 1,
        WM_SIZE => {
            let mut rc = RectWin {
                left: 0,
                top: 0,
                right: 1360,
                bottom: 860,
            };
            GetClientRect(hwnd, &mut rc);
            ctx.lienzo
                .redimensionar(rc.right - rc.left, rc.bottom - rc.top);
            InvalidateRect(hwnd, ptr::null(), 0);
            0
        }
        WM_TIMER => {
            InvalidateRect(hwnd, ptr::null(), 0);
            0
        }
        WM_LBUTTONDOWN => {
            let mx = (l_param & 0xFFFF) as i16 as i32;
            let my = ((l_param >> 16) & 0xFFFF) as i16 as i32;
            SetCapture(hwnd);
            ctx.manejar_clic_izquierdo(mx, my);
            InvalidateRect(hwnd, ptr::null(), 0);
            0
        }
        WM_MOUSEMOVE => {
            if ctx.control_arrastrado != ControlArrastrado::Ninguno {
                let mx = (l_param & 0xFFFF) as i16 as i32;
                let my = ((l_param >> 16) & 0xFFFF) as i16 as i32;
                ctx.actualizar_arrastre_raton(mx, my);
                InvalidateRect(hwnd, ptr::null(), 0);
            }
            0
        }
        WM_LBUTTONUP => {
            if ctx.control_arrastrado != ControlArrastrado::Ninguno {
                ctx.control_arrastrado = ControlArrastrado::Ninguno;
                ReleaseCapture();
                InvalidateRect(hwnd, ptr::null(), 0);
            }
            0
        }
        WM_RBUTTONDOWN => {
            let mx = (l_param & 0xFFFF) as i16 as i32;
            let my = ((l_param >> 16) & 0xFFFF) as i16 as i32;
            ctx.manejar_clic_derecho(mx, my);
            InvalidateRect(hwnd, ptr::null(), 0);
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((w_param >> 16) & 0xFFFF) as i16;
            ctx.manejar_rueda_raton(delta);
            InvalidateRect(hwnd, ptr::null(), 0);
            0
        }
        WM_CHAR => {
            ctx.manejar_caracter_teclado(w_param as u32);
            InvalidateRect(hwnd, ptr::null(), 0);
            0
        }
        WM_KEYDOWN => {
            // Teclas 1, 2, 3, 4 para cambiar rapidamente de escena si no se esta escribiendo en AutoEQ
            if !ctx.buscador_autoeq_enfocado {
                match w_param {
                    0x31 => ctx.escena_actual = EscenaPoweramp::EcualizadorYTono,
                    0x32 => ctx.escena_actual = EscenaPoweramp::EspacioYCompresor,
                    0x33 => ctx.escena_actual = EscenaPoweramp::Reverberacion,
                    0x34 => ctx.escena_actual = EscenaPoweramp::AutoEqYDispositivos,
                    _ => {}
                }
                InvalidateRect(hwnd, ptr::null(), 0);
            }
            0
        }
        WM_PAINT => {
            let mut ps = PaintStruct {
                hdc: ptr::null_mut(),
                f_erase: 0,
                rc_paint: RectWin {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                f_restore: 0,
                f_inc_update: 0,
                rgb_reserved: [0; 32],
            };
            let hdc_ventana = BeginPaint(hwnd, &mut ps);
            if !hdc_ventana.is_null() {
                ctx.renderizar_cuadro_completo();
                BitBlt(
                    hdc_ventana,
                    0,
                    0,
                    ctx.lienzo.ancho,
                    ctx.lienzo.alto,
                    ctx.lienzo.hdc_memoria,
                    0,
                    0,
                    SRCCOPY,
                );
            }
            EndPaint(hwnd, &ps);
            0
        }
        WM_CLOSE | WM_DESTROY => {
            ctx.telemetria.ejecutando.store(false, Ordering::Relaxed);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

/// Abre y ejecuta el bucle de eventos de la ventana nativa Win32/GDI de Poweramp Equalizer en Rust.
pub fn ejecutar_ventana_nativa_poweramp(
    cadena_dsp: Arc<Mutex<CadenaMaestraDsp>>,
    telemetria: Arc<TelemetriaCompartidaVentana>,
    enrutamiento: Arc<Mutex<EnrutamientoWasapiEstado>>,
    directorio_base: PathBuf,
) -> Result<(), String> {
    unsafe {
        let mut caja_ctx = Box::new(ContextoVentanaPoweramp::nuevo(
            cadena_dsp,
            telemetria,
            enrutamiento,
            directorio_base,
        ));
        if let Ok(c) = caja_ctx.cadena_dsp.lock() {
            caja_ctx.sincronizar_persistencia_dsp(c.obtener_estado());
        }
        PUNTERO_CONTEXTO_GLOBAL = &mut *caja_ctx as *mut ContextoVentanaPoweramp;

        let h_instance = GetModuleHandleW(ptr::null());
        let nombre_clase = a_utf16_nulo("ClaseVentanaPowerampRustNativa");
        let titulo_ventana = a_utf16_nulo(
            "Poweramp Equalizer — Motor DSP Soberano en Rust (64-bit Biquad + WASAPI + AutoEQ)",
        );

        let wc = WndClassExW {
            cb_size: std::mem::size_of::<WndClassExW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
            lpfn_wnd_proc: wnd_proc_poweramp,
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance,
            h_icon: ptr::null_mut(),
            h_cursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
            hbr_background: ptr::null_mut(),
            lpsz_menu_name: ptr::null(),
            lpsz_class_name: nombre_clase.as_ptr(),
            h_icon_sm: ptr::null_mut(),
        };

        RegisterClassExW(&wc);

        let hwnd = CreateWindowExW(
            0,
            nombre_clase.as_ptr(),
            titulo_ventana.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            80,
            50,
            1380,
            890,
            ptr::null_mut(),
            ptr::null_mut(),
            h_instance,
            ptr::null_mut(),
        );

        if hwnd.is_null() {
            PUNTERO_CONTEXTO_GLOBAL = ptr::null_mut();
            return Err(String::from(
                "No se pudo crear la ventana nativa Win32 de Poweramp Equalizer",
            ));
        }

        ShowWindow(hwnd, 5);
        UpdateWindow(hwnd);
        // Temporizador de refresco de 60 FPS (16 ms)
        SetTimer(hwnd, ID_TEMPORIZADOR_60FPS, 16, 0);

        let mut msg = Msg {
            hwnd: ptr::null_mut(),
            message: 0,
            w_param: 0,
            l_param: 0,
            time: 0,
            pt: Point { x: 0, y: 0 },
        };

        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        PUNTERO_CONTEXTO_GLOBAL = ptr::null_mut();
        Ok(())
    }
}
