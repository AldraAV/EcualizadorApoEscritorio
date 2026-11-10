//! Compresor Dinamico Estereo con filtro de cadena lateral (Sidechain) y Limitador Maestro.
//! Implementa de forma nativa los parametros de `peq_compr.xml` de Poweramp Equalizer
//! (Limite, Proporcion, Ataque, Lanzamiento, Frecuencia Sidechain y Ganancia de Maquillaje)
//! sin depender de plugins VST externos.

use crate::filtro_biquad::FiltroBiquadEstereo;
use crate::tipos_audio::{ParametrosCompresor, TipoFiltro};

/// Compresor dinamico estereo de rodilla suave (Soft-Knee) con detector sidechain filtrado.
#[derive(Debug, Clone)]
pub struct CompresorDinamicoEstereo {
    parametros: ParametrosCompresor,
    frecuencia_muestreo: f64,
    coeficiente_ataque: f64,
    coeficiente_lanzamiento: f64,
    ganancia_lineal_maquillaje: f64,
    envolvente_db: f64,
    reduccion_actual_db: f64,
    filtro_sidechain: FiltroBiquadEstereo,
}

impl CompresorDinamicoEstereo {
    pub fn nuevo(frecuencia_muestreo: f64) -> Self {
        let mut compresor = Self {
            parametros: ParametrosCompresor::default(),
            frecuencia_muestreo: frecuencia_muestreo.max(8000.0),
            coeficiente_ataque: 0.0,
            coeficiente_lanzamiento: 0.0,
            ganancia_lineal_maquillaje: 1.0,
            envolvente_db: -96.0,
            reduccion_actual_db: 0.0,
            filtro_sidechain: FiltroBiquadEstereo::nuevo(),
        };
        compresor.recalcular_constantes();
        compresor
    }

    pub fn establecer_frecuencia_muestreo(&mut self, nueva_frecuencia_muestreo: f64) {
        self.frecuencia_muestreo = nueva_frecuencia_muestreo.max(8000.0);
        self.recalcular_constantes();
    }

    pub fn actualizar_parametros(&mut self, nuevos_parametros: ParametrosCompresor) {
        self.parametros = nuevos_parametros;
        self.recalcular_constantes();
    }

    pub fn obtener_parametros(&self) -> ParametrosCompresor {
        self.parametros
    }

    pub fn obtener_reduccion_db(&self) -> f64 {
        self.reduccion_actual_db
    }

    fn recalcular_constantes(&mut self) {
        let ataque_seg = (self.parametros.ataque_ms.clamp(0.1, 500.0)) * 0.001;
        let lanzamiento_seg = (self.parametros.lanzamiento_ms.clamp(5.0, 5000.0)) * 0.001;

        self.coeficiente_ataque = (-1.0 / (ataque_seg * self.frecuencia_muestreo)).exp();
        self.coeficiente_lanzamiento = (-1.0 / (lanzamiento_seg * self.frecuencia_muestreo)).exp();
        self.ganancia_lineal_maquillaje = 10.0_f64.powf(self.parametros.ganancia_maquillaje_db / 20.0);

        let frec_sc = self.parametros.frecuencia_sidechain_hz.clamp(20.0, 4000.0);
        let sc_activo = frec_sc > 22.0;
        let _ = self.filtro_sidechain.configurar_parametros(
            frec_sc,
            0.707,
            0.0,
            TipoFiltro::PasoAlto,
            self.frecuencia_muestreo,
            sc_activo,
        );
    }

    /// Procesa un par estereo aplicando deteccion de envolvente con filtro sidechain y rodilla suave.
    #[inline(always)]
    pub fn procesar_par_estereo(&mut self, muestra_izq: f64, muestra_der: f64) -> (f64, f64) {
        if !self.parametros.activo || self.parametros.proporcion <= 1.01 {
            self.reduccion_actual_db = 0.0;
            return (muestra_izq, muestra_der);
        }

        // 1. Filtrar la senal de deteccion por el filtro Paso-Alto de cadena lateral (Sidechain)
        let (sc_izq, sc_der) = self.filtro_sidechain.procesar_par_estereo(muestra_izq, muestra_der);

        // 2. Detector de pico vinculado en estereo
        let nivel_pico = sc_izq.abs().max(sc_der.abs()).max(1e-6);
        let nivel_entrada_db = 20.0 * nivel_pico.log10();

        // 3. Seguidor de envolvente balistico (Ataque / Lanzamiento)
        if nivel_entrada_db > self.envolvente_db {
            self.envolvente_db = self.coeficiente_ataque * self.envolvente_db
                + (1.0 - self.coeficiente_ataque) * nivel_entrada_db;
        } else {
            self.envolvente_db = self.coeficiente_lanzamiento * self.envolvente_db
                + (1.0 - self.coeficiente_lanzamiento) * nivel_entrada_db;
        }

        // 4. Computo de ganancia con rodilla suave (Soft-Knee de 6 dB)
        let ancho_rodilla = 6.0;
        let exceso_db = self.envolvente_db - self.parametros.limite_db;
        let proporcion = self.parametros.proporcion.max(1.0);

        let reduccion_db = if exceso_db <= -ancho_rodilla * 0.5 {
            0.0
        } else if exceso_db >= ancho_rodilla * 0.5 {
            exceso_db * (1.0 - 1.0 / proporcion)
        } else {
            let factor_intermedio = exceso_db + ancho_rodilla * 0.5;
            (1.0 - 1.0 / proporcion) * (factor_intermedio * factor_intermedio) / (2.0 * ancho_rodilla)
        };

        self.reduccion_actual_db = reduccion_db;
        let ganancia_reduccion_lineal = 10.0_f64.powf(-reduccion_db / 20.0) * self.ganancia_lineal_maquillaje;

        (
            muestra_izq * ganancia_reduccion_lineal,
            muestra_der * ganancia_reduccion_lineal,
        )
    }
}

/// Limitador Maestro Anti-Distorsion de baja latencia (Lookahead / Soft-Clipper hibrido).
#[derive(Debug, Clone)]
pub struct LimitadorMaestroEstereo {
    envolvente_pico: f64,
    coeficiente_liberacion: f64,
    umbral_techo: f64,
}

impl Default for LimitadorMaestroEstereo {
    fn default() -> Self {
        Self::nuevo(48000.0)
    }
}

impl LimitadorMaestroEstereo {
    pub fn nuevo(frecuencia_muestreo: f64) -> Self {
        let fs = frecuencia_muestreo.max(8000.0);
        Self {
            envolvente_pico: 0.0,
            coeficiente_liberacion: (-1.0 / (0.050 * fs)).exp(),
            // Techo en -0.1 dBFS (~0.9885)
            umbral_techo: 0.9885,
        }
    }

    #[inline(always)]
    pub fn procesar_par_estereo(&mut self, muestra_izq: f64, muestra_der: f64, activo: bool) -> (f64, f64) {
        if !activo {
            return (
                muestra_izq.clamp(-1.0, 1.0),
                muestra_der.clamp(-1.0, 1.0),
            );
        }

        let pico_actual = muestra_izq.abs().max(muestra_der.abs());
        if pico_actual > self.envolvente_pico {
            self.envolvente_pico = pico_actual;
        } else {
            self.envolvente_pico = self.coeficiente_liberacion * self.envolvente_pico
                + (1.0 - self.coeficiente_liberacion) * pico_actual;
        }

        let ganancia_limitador = if self.envolvente_pico > self.umbral_techo {
            self.umbral_techo / self.envolvente_pico
        } else {
            1.0
        };

        let izq_limitada = (muestra_izq * ganancia_limitador).clamp(-1.0, 1.0);
        let der_limitada = (muestra_der * ganancia_limitador).clamp(-1.0, 1.0);

        (izq_limitada, der_limitada)
    }
}
