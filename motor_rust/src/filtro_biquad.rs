//! Filtro IIR Biquad de segundo orden en precision doble (64 bits) con topologia
//! Direct Form II Transposed, suavizado exponencial de coeficientes (cero zipper noise)
//! y evaluacion de magnitud exacta basada en `Coefs.smali` de Poweramp Equalizer.

use std::f64::consts::PI;

use crate::tipos_audio::{BandaParametrica, CanalAplicacion, ErrorMotorAudio, TipoFiltro};

/// Coeficientes normalizados (con a0 = 1.0) de un filtro Biquad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoeficientesBiquad {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl Default for CoeficientesBiquad {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

/// Filtro Biquad estereo de alta precision con interpolacion suave de coeficientes.
#[derive(Debug, Clone)]
pub struct FiltroBiquadEstereo {
    coeficientes_actuales: CoeficientesBiquad,
    coeficientes_objetivo: CoeficientesBiquad,
    factor_suavizado: f64,
    // Registros de retardo Direct Form II Transposed (Canal Izquierdo)
    estado_z1_izq: f64,
    estado_z2_izq: f64,
    // Registros de retardo Direct Form II Transposed (Canal Derecho)
    estado_z1_der: f64,
    estado_z2_der: f64,
    canales: CanalAplicacion,
    esta_activo: bool,
    inicializado: bool,
}

impl Default for FiltroBiquadEstereo {
    fn default() -> Self {
        Self::nuevo()
    }
}

impl FiltroBiquadEstereo {
    /// Crea un nuevo filtro biquad estereo en estado neutro (paso transparente).
    pub fn nuevo() -> Self {
        Self {
            coeficientes_actuales: CoeficientesBiquad::default(),
            coeficientes_objetivo: CoeficientesBiquad::default(),
            // Constante de suavizado por muestra (~5 ms a 48 kHz) para evitar chasquidos al mover faders
            factor_suavizado: 0.008,
            estado_z1_izq: 0.0,
            estado_z2_izq: 0.0,
            estado_z1_der: 0.0,
            estado_z2_der: 0.0,
            canales: CanalAplicacion::EstereoAmbos,
            esta_activo: false,
            inicializado: false,
        }
    }

    /// Reinicia los registros de memoria z^-1 y z^-2 a cero para evitar transitorios.
    #[inline]
    pub fn reiniciar_memoria(&mut self) {
        self.estado_z1_izq = 0.0;
        self.estado_z2_izq = 0.0;
        self.estado_z1_der = 0.0;
        self.estado_z2_der = 0.0;
    }

    /// Calcula los coeficientes normalizados RBJ / Poweramp para una banda y frecuencia de muestreo.
    pub fn calcular_coeficientes(
        frecuencia_hz: f64,
        factor_q: f64,
        ganancia_db: f64,
        tipo: TipoFiltro,
        frecuencia_muestreo: f64,
    ) -> Result<CoeficientesBiquad, ErrorMotorAudio> {
        if frecuencia_muestreo <= 0.0 || !frecuencia_muestreo.is_finite() {
            return Err(ErrorMotorAudio::FrecuenciaMuestreoInvalida(frecuencia_muestreo));
        }

        let nyquist_hz = frecuencia_muestreo * 0.5;
        if frecuencia_hz <= 1.0 || frecuencia_hz >= nyquist_hz {
            return Err(ErrorMotorAudio::FrecuenciaFueraDeRango {
                frecuencia_hz,
                nyquist_hz,
            });
        }

        if factor_q <= 0.0 || !factor_q.is_finite() {
            return Err(ErrorMotorAudio::FactorQInvalido(factor_q));
        }

        let omega = 2.0 * PI * frecuencia_hz / frecuencia_muestreo;
        let seno_omega = omega.sin();
        let coseno_omega = omega.cos();
        let alfa = seno_omega / (2.0 * factor_q);
        let factor_a = 10.0_f64.powf(ganancia_db / 40.0);

        let (b0_crudo, b1_crudo, b2_crudo, a0_crudo, a1_crudo, a2_crudo) = match tipo {
            TipoFiltro::PicoParametrico => (
                1.0 + alfa * factor_a,
                -2.0 * coseno_omega,
                1.0 - alfa * factor_a,
                1.0 + alfa / factor_a,
                -2.0 * coseno_omega,
                1.0 - alfa / factor_a,
            ),
            TipoFiltro::EstanteGraves => {
                let dos_raiz_a_alfa = 2.0 * factor_a.sqrt() * alfa;
                (
                    factor_a * ((factor_a + 1.0) - (factor_a - 1.0) * coseno_omega + dos_raiz_a_alfa),
                    2.0 * factor_a * ((factor_a - 1.0) - (factor_a + 1.0) * coseno_omega),
                    factor_a * ((factor_a + 1.0) - (factor_a - 1.0) * coseno_omega - dos_raiz_a_alfa),
                    (factor_a + 1.0) + (factor_a - 1.0) * coseno_omega + dos_raiz_a_alfa,
                    -2.0 * ((factor_a - 1.0) + (factor_a + 1.0) * coseno_omega),
                    (factor_a + 1.0) + (factor_a - 1.0) * coseno_omega - dos_raiz_a_alfa,
                )
            }
            TipoFiltro::EstanteAgudos => {
                let dos_raiz_a_alfa = 2.0 * factor_a.sqrt() * alfa;
                (
                    factor_a * ((factor_a + 1.0) + (factor_a - 1.0) * coseno_omega + dos_raiz_a_alfa),
                    -2.0 * factor_a * ((factor_a - 1.0) + (factor_a + 1.0) * coseno_omega),
                    factor_a * ((factor_a + 1.0) + (factor_a - 1.0) * coseno_omega - dos_raiz_a_alfa),
                    (factor_a + 1.0) - (factor_a - 1.0) * coseno_omega + dos_raiz_a_alfa,
                    2.0 * ((factor_a - 1.0) - (factor_a + 1.0) * coseno_omega),
                    (factor_a + 1.0) - (factor_a - 1.0) * coseno_omega - dos_raiz_a_alfa,
                )
            }
            TipoFiltro::PasoBajo => (
                (1.0 - coseno_omega) * 0.5,
                1.0 - coseno_omega,
                (1.0 - coseno_omega) * 0.5,
                1.0 + alfa,
                -2.0 * coseno_omega,
                1.0 - alfa,
            ),
            TipoFiltro::PasoAlto => (
                (1.0 + coseno_omega) * 0.5,
                -(1.0 + coseno_omega),
                (1.0 + coseno_omega) * 0.5,
                1.0 + alfa,
                -2.0 * coseno_omega,
                1.0 - alfa,
            ),
            TipoFiltro::PasoBanda => (
                alfa,
                0.0,
                -alfa,
                1.0 + alfa,
                -2.0 * coseno_omega,
                1.0 - alfa,
            ),
        };

        if a0_crudo.abs() < 1e-15 || !a0_crudo.is_finite() {
            return Err(ErrorMotorAudio::InestabilidadNumericaBiquad);
        }

        let inverso_a0 = 1.0 / a0_crudo;
        Ok(CoeficientesBiquad {
            b0: b0_crudo * inverso_a0,
            b1: b1_crudo * inverso_a0,
            b2: b2_crudo * inverso_a0,
            a1: a1_crudo * inverso_a0,
            a2: a2_crudo * inverso_a0,
        })
    }

    /// Configura o actualiza una banda aplicando suavizado sin saltos abruptos.
    pub fn configurar_banda(
        &mut self,
        banda: &BandaParametrica,
        frecuencia_muestreo: f64,
    ) -> Result<(), ErrorMotorAudio> {
        self.esta_activo = banda.activo;
        self.canales = banda.canales;

        if !self.esta_activo {
            self.coeficientes_objetivo = CoeficientesBiquad::default();
            if !self.inicializado {
                self.coeficientes_actuales = self.coeficientes_objetivo;
                self.inicializado = true;
            }
            return Ok(());
        }

        // Acotar frecuencia por seguridad a 0.495 * Fs si un preset de 96kHz se carga en 48kHz
        let frecuencia_segura = banda
            .frecuencia_hz
            .clamp(10.0, frecuencia_muestreo * 0.495);
        let q_seguro = banda.factor_q.clamp(0.05, 30.0);

        let nuevos = Self::calcular_coeficientes(
            frecuencia_segura,
            q_seguro,
            banda.ganancia_db,
            banda.tipo,
            frecuencia_muestreo,
        )?;

        self.coeficientes_objetivo = nuevos;
        if !self.inicializado {
            self.coeficientes_actuales = nuevos;
            self.inicializado = true;
        }

        Ok(())
    }

    /// Configura directamente con parametros escalares sin requerir estructura BandaParametrica.
    pub fn configurar_parametros(
        &mut self,
        frecuencia_hz: f64,
        factor_q: f64,
        ganancia_db: f64,
        tipo: TipoFiltro,
        frecuencia_muestreo: f64,
        activo: bool,
    ) -> Result<(), ErrorMotorAudio> {
        let banda = BandaParametrica {
            identificador: 0,
            frecuencia_hz,
            factor_q,
            ganancia_db,
            tipo,
            canales: CanalAplicacion::EstereoAmbos,
            activo,
            color_argb: 0,
        };
        self.configurar_banda(&banda, frecuencia_muestreo)
    }

    /// Procesa un par de muestras estereo (Izquierda, Derecha) usando Direct Form II Transposed
    /// con suavizado de coeficientes y prevencion de numeros subnormales (denormals).
    #[inline(always)]
    pub fn procesar_par_estereo(&mut self, mut muestra_izq: f64, mut muestra_der: f64) -> (f64, f64) {
        if !self.esta_activo {
            return (muestra_izq, muestra_der);
        }

        // Interpolacion exponencial hacia los coeficientes objetivo para eliminar zipper noise
        let s = self.factor_suavizado;
        self.coeficientes_actuales.b0 += (self.coeficientes_objetivo.b0 - self.coeficientes_actuales.b0) * s;
        self.coeficientes_actuales.b1 += (self.coeficientes_objetivo.b1 - self.coeficientes_actuales.b1) * s;
        self.coeficientes_actuales.b2 += (self.coeficientes_objetivo.b2 - self.coeficientes_actuales.b2) * s;
        self.coeficientes_actuales.a1 += (self.coeficientes_objetivo.a1 - self.coeficientes_actuales.a1) * s;
        self.coeficientes_actuales.a2 += (self.coeficientes_objetivo.a2 - self.coeficientes_actuales.a2) * s;

        let c = self.coeficientes_actuales;

        if self.canales == CanalAplicacion::EstereoAmbos || self.canales == CanalAplicacion::SoloIzquierdo {
            let salida_izq = c.b0 * muestra_izq + self.estado_z1_izq;
            self.estado_z1_izq = c.b1 * muestra_izq - c.a1 * salida_izq + self.estado_z2_izq;
            self.estado_z2_izq = c.b2 * muestra_izq - c.a2 * salida_izq;
            if self.estado_z1_izq.abs() < 1e-24 {
                self.estado_z1_izq = 0.0;
            }
            if self.estado_z2_izq.abs() < 1e-24 {
                self.estado_z2_izq = 0.0;
            }
            muestra_izq = salida_izq;
        }

        if self.canales == CanalAplicacion::EstereoAmbos || self.canales == CanalAplicacion::SoloDerecho {
            let salida_der = c.b0 * muestra_der + self.estado_z1_der;
            self.estado_z1_der = c.b1 * muestra_der - c.a1 * salida_der + self.estado_z2_der;
            self.estado_z2_der = c.b2 * muestra_der - c.a2 * salida_der;
            if self.estado_z1_der.abs() < 1e-24 {
                self.estado_z1_der = 0.0;
            }
            if self.estado_z2_der.abs() < 1e-24 {
                self.estado_z2_der = 0.0;
            }
            muestra_der = salida_der;
        }

        (muestra_izq, muestra_der)
    }

    /// Evalua la magnitud lineal |H(e^{j\omega})| en una frecuencia especifica
    /// utilizando la ecuacion exacta extraida de `Coefs.smali -> В()` de Poweramp.
    pub fn evaluar_magnitud(&self, frecuencia_hz: f64, frecuencia_muestreo: f64) -> f64 {
        if !self.esta_activo || frecuencia_muestreo <= 0.0 {
            return 1.0;
        }

        let c = self.coeficientes_objetivo;
        let omega = 2.0 * PI * frecuencia_hz / frecuencia_muestreo;
        let seno_mitad_omega = (omega * 0.5).sin();
        let ese = seno_mitad_omega * seno_mitad_omega;

        let suma_b = c.b0 + c.b1 + c.b2;
        let suma_a = 1.0 + c.a1 + c.a2;

        let num_base = suma_b * suma_b;
        let prod_b = c.b0 * c.b1 + 4.0 * c.b0 * c.b2 + c.b1 * c.b2;
        let num = num_base - 4.0 * prod_b * ese + 16.0 * c.b0 * c.b2 * ese * ese;

        let den_base = suma_a * suma_a;
        let prod_a = c.a1 + 4.0 * c.a2 + c.a1 * c.a2;
        let den = den_base - 4.0 * prod_a * ese + 16.0 * c.a2 * ese * ese;

        if den.abs() < 1e-15 {
            return 0.0;
        }

        let cociente = (num / den).max(0.0);
        cociente.sqrt()
    }
}
