//! Crate principal `motor_ecualizador_poweramp` (Motor DSP Soberano en Rust).
//! Implementa de forma nativa toda la arquitectura acustica extraida de Poweramp Equalizer:
//! - Filtros IIR Biquad de 64 bits con formula de magnitud `Coefs.smali`.
//! - Controles de Tono (Graves y Agudos), Preamplificador y Limitador Maestro.
//! - Procesador de Espacio y Tiempo (Balance, Expansion Estereo Mid/Side, Mono, Tempo).
//! - Compresor Dinamico Estereo con filtro de cadena lateral (Sidechain).
//! - Reverberacion Estereo algorítmica Schroeder-Moorer / Freeverb.
//! - Analizador de Espectro FFT Cooley-Tukey Radix-2 de 1024 puntos (cero asignaciones) con suavizado `milk_main.lua`.
//! - Decodificador nativo de presets `.json`, `.peq-settings` y blobs binarios `PaP\x01` (`auto_eq.db`).

pub mod analizador_espectro;
pub mod apo_com_windows;
pub mod cadena_maestra_dsp;
pub mod captura_wasapi_loopback;
pub mod cargador_presets;
pub mod compresor_dinamico;
pub mod extractor_ui_apk;
pub mod filtro_biquad;
pub mod instalador_apo;
pub mod interfaz_iced;
pub mod procesador_espacio_reverb;
pub mod tipos_audio;
pub mod ventana_nativa_win32;

#[cfg(test)]
mod pruebas_unitarias {
    use std::f32::consts::PI;

    use crate::analizador_espectro::AnalizadorEspectroTiempoReal;
    use crate::cadena_maestra_dsp::CadenaMaestraDsp;
    use crate::cargador_presets::{
        decodificar_blob_pap, generar_configuracion_apo, parsear_preset_json_poweramp,
    };
    use crate::filtro_biquad::FiltroBiquadEstereo;
    use crate::tipos_audio::{BandaParametrica, ParametrosCompresor, ParametrosEspacio, TipoFiltro};

    #[test]
    fn prueba_biquad_evaluacion_magnitud_coefs_smali() {
        // Preparar (Arrange): Banda parametrica de +12 dB en 1000 Hz a 48 kHz
        let mut filtro = FiltroBiquadEstereo::nuevo();
        let banda = BandaParametrica::nueva(1, 1000.0, 1.41, 12.0, TipoFiltro::PicoParametrico);
        filtro
            .configurar_banda(&banda, 48000.0)
            .expect("Configuracion de banda biquad exitosa");

        // Actuar (Act): Evaluar magnitud en el centro (1000 Hz) y en extremo grave (25 Hz)
        let mag_centro = filtro.evaluar_magnitud(1000.0, 48000.0);
        let db_centro = 20.0 * mag_centro.log10();
        let mag_grave = filtro.evaluar_magnitud(25.0, 48000.0);
        let db_grave = 20.0 * mag_grave.log10();

        // Verificar (Assert): En 1000 Hz debe haber +12.0 dB exactos, y en 25 Hz ~0.0 dB
        assert!(
            (db_centro - 12.0).abs() < 0.05,
            "La ganancia en 1000 Hz debia ser 12.0 dB pero fue {} dB",
            db_centro
        );
        assert!(
            db_grave.abs() < 0.1,
            "La ganancia fuera de banda en 25 Hz debia ser cercana a 0 dB pero fue {} dB",
            db_grave
        );
    }

    #[test]
    fn prueba_decodificacion_blob_binario_pap_autoeq() {
        // Preparar (Arrange): Blob real extraido de auto_eq.db para "1MORE ComfoBuds Mini" (2 primeras bandas)
        let blob_hex = "506150010200010000700000000000009a99193f5a00000001700000000000003333933f1027000004700000cdcc4c3e5f0000003333333fff303fff037000003333b3bf8f0000006666e63fff303fff";
        let mut bytes = Vec::with_capacity(blob_hex.len() / 2);
        for i in (0..blob_hex.len()).step_by(2) {
            let byte = u8::from_str_radix(&blob_hex[i..i + 2], 16).unwrap();
            bytes.push(byte);
        }

        // Actuar (Act)
        let estado = decodificar_blob_pap("1MORE ComfoBuds Mini", &bytes)
            .expect("Decodificacion de blob PaP\\x01 exitosa");

        // Verificar (Assert)
        assert_eq!(estado.nombre_perfil, "1MORE ComfoBuds Mini");
        assert!(estado.modo_parametrico);
        assert_eq!(estado.bandas.len(), 2);
        assert_eq!(estado.bandas[0].tipo, TipoFiltro::EstanteGraves);
        assert!((estado.bandas[0].frecuencia_hz - 95.0).abs() < 0.01);
        assert!((estado.bandas[0].ganancia_db - 0.2).abs() < 0.01);
        assert!((estado.bandas[0].factor_q - 0.7).abs() < 0.01);

        assert_eq!(estado.bandas[1].tipo, TipoFiltro::PicoParametrico);
        assert!((estado.bandas[1].frecuencia_hz - 143.0).abs() < 0.01);
        assert!((estado.bandas[1].ganancia_db - (-1.4)).abs() < 0.01);
    }

    #[test]
    fn prueba_parseo_json_poweramp_jbl_tune_720bt() {
        // Preparar (Arrange): Estructura JSON de Poweramp Equalizer
        let json_ejemplo = r#"[{
            "name": "JBL Tune 720BT (passive)",
            "preamp": 0.0,
            "parametric": true,
            "bands": [
                { "type": 0, "channels": 0, "frequency": 90, "q": 0.8, "gain": 0.0, "color": 0 },
                { "type": 1, "channels": 0, "frequency": 10000, "q": 0.6, "gain": 0.0, "color": 0 },
                { "type": 4, "channels": 0, "frequency": 105, "q": 0.7, "gain": 5.3, "color": -12974645 },
                { "type": 3, "channels": 0, "frequency": 192, "q": 1.62, "gain": -5.6, "color": -5804289 }
            ]
        }]"#;

        // Actuar (Act)
        let estado = parsear_preset_json_poweramp(json_ejemplo)
            .expect("Parseo de JSON Poweramp exitoso");

        // Verificar (Assert)
        assert_eq!(estado.nombre_perfil, "JBL Tune 720BT (passive)");
        assert_eq!(estado.bandas.len(), 2);
        assert_eq!(estado.bandas[0].tipo, TipoFiltro::EstanteGraves);
        assert!((estado.bandas[0].frecuencia_hz - 105.0).abs() < 0.01);
        assert!((estado.bandas[0].ganancia_db - 5.3).abs() < 0.01);

        let config_apo = generar_configuracion_apo(&estado);
        assert!(config_apo.contains("Filter 1: ON LS Fc 105.0 Hz Gain 5.30 dB Q 0.70"));
    }

    #[test]
    fn prueba_cadena_maestra_compresor_y_espacio() {
        // Preparar (Arrange)
        let mut cadena = CadenaMaestraDsp::nueva(48000.0);
        cadena.actualizar_compresor(ParametrosCompresor {
            activo: true,
            limite_db: -18.0,
            proporcion: 4.0,
            ataque_ms: 0.5,
            lanzamiento_ms: 50.0,
            frecuencia_sidechain_hz: 60.0,
            ganancia_maquillaje_db: 0.0,
        });
        cadena.actualizar_espacio(ParametrosEspacio {
            balance: 0.0,
            expansion_estereo: 0.5,
            modo_mono: false,
        });

        // Actuar (Act): Alimentar senal de 1 kHz de alta amplitud (0.8)
        for i in 0..1024 {
            let fase = 2.0 * PI * 1000.0 * (i as f32) / 48000.0;
            let m_izq = 0.8 * fase.sin();
            let m_der = 0.8 * fase.cos();
            let (s_izq, s_der) = cadena.procesar_par_estereo(m_izq, m_der);
            assert!(s_izq.is_finite() && s_der.is_finite());
        }

        // Verificar (Assert): El compresor debe reportar reduccion de ganancia positiva
        assert!(
            cadena.obtener_reduccion_compresor_db() > 1.0,
            "El compresor debia reducir ganancia sobre una senal que supera -18 dBFS"
        );
    }

    #[test]
    fn prueba_analizador_espectro_fft_cero_asignaciones() {
        // Preparar (Arrange)
        let mut analizador = AnalizadorEspectroTiempoReal::nuevo(0.5);

        // Actuar (Act): Alimentar 1024 muestras de tono senoidal puro de 1 kHz a 48 kHz
        let mut hubo_cuadro = false;
        for i in 0..1024 {
            let muestra = (2.0 * PI * 1000.0 * (i as f32) / 48000.0).sin();
            if analizador.alimentar_muestra_estereo(muestra, muestra) {
                hubo_cuadro = true;
            }
        }

        // Verificar (Assert)
        assert!(hubo_cuadro, "El analizador FFT debio producir al menos un cuadro tras 1024 muestras");
        let barras = analizador.obtener_barras();
        let energia_maxima = barras.iter().copied().fold(0.0_f32, f32::max);
        assert!(energia_maxima > 0.2, "Las barras del espectro deben reflejar la energia del tono de 1 kHz");
    }

    #[test]
    fn prueba_busqueda_real_autoeq_db() {
        let ruta_db = std::path::Path::new("../recursos/auto_eq.db");
        if !ruta_db.exists() {
            return;
        }
        let resultados = crate::cargador_presets::buscar_presets_autoeq_en_archivo(ruta_db, "JBL Tune", 10)
            .expect("Busqueda en auto_eq.db exitosa");
        assert!(!resultados.is_empty(), "Debe encontrar audifonos JBL Tune en auto_eq.db");

        let estado = crate::cargador_presets::cargar_preset_autoeq_por_consulta(ruta_db, "JBL Tune 720BT")
            .expect("Carga de perfil JBL Tune 720BT desde auto_eq.db exitosa");
        assert!(!estado.bandas.is_empty());
    }

    #[test]
    fn prueba_decodificacion_layouts_interfaz_apk_rea() {
        let dir_rea = std::path::Path::new(r"C:\Users\carde\AppData\Local\Temp\rea-extracted-a10b98e6-03e5-4732-b4a1-7df4e3a65808");
        if !dir_rea.exists() {
            return;
        }
        let archivos = [
            "res/layout-sw4dp-v13/activity_peq_main.xml",
            "res/layout-sw2dp-v13/merge_eq_layout.xml",
            "res/layout-sw2dp-v13/merge_eq_equalizer_tab_common.xml",
            "res/layout-sw4dp-v13/merge_eq_equalizer_buttons.xml",
            "res/layout-sw2dp-v13/merge_eq_vol_tab.xml",
            "res/layout-sw2dp-v13/merge_eq_reverb_tab.xml",
            "res/layout-sw2dp-v13/merge_eqbuttons_edit.xml",
            "res/layout-sw2dp-v13/merge_navbar.xml",
            "res/layout-sw2dp-v13/merge_vis_panel.xml",
            "res/xml/peq_builtin_skins.xml",
            "res/xml/peq_equ.xml",
            "res/xml/peq_compr.xml",
        ];
        let mut acumulado = String::new();
        for rel in archivos {
            let ruta = dir_rea.join(rel);
            if let Ok(xml) = crate::extractor_ui_apk::decodificar_archivo_axml(&ruta) {
                acumulado.push_str(&format!("<!-- ===== {} ===== -->\n{}\n", rel, xml));
            }
        }
        let _ = std::fs::write("../recursos/layouts_poweramp_decodificados.xml", &acumulado);
        assert!(!acumulado.is_empty());
    }
}
