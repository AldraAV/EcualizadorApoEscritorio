//! Procesadores de Espacio (Balance, Expansion Estereo Mid/Side, Mono)
//! y Reverberacion Estereo algorítmica Schroeder-Moorer / Freeverb con Preretardo,
//! Amortiguacion, Tamano de Sala, Filtro Tonal y Mezclado Dry/Wet.
//! Cero asignaciones dinamicas durante el procesamiento en tiempo real.

use std::f64::consts::FRAC_PI_4;

use crate::filtro_biquad::FiltroBiquadEstereo;
use crate::tipos_audio::{ParametrosEspacio, ParametrosReverberacion, TipoFiltro};

/// Procesador espacial estereo estilo Poweramp Equalizer.
#[derive(Debug, Clone)]
pub struct ProcesadorEspacioTiempo {
    parametros: ParametrosEspacio,
    ganancia_balance_izq: f64,
    ganancia_balance_der: f64,
}

impl Default for ProcesadorEspacioTiempo {
    fn default() -> Self {
        Self::nuevo(48000.0)
    }
}

impl ProcesadorEspacioTiempo {
    pub fn nuevo(_frecuencia_muestreo: f64) -> Self {
        let mut proc = Self {
            parametros: ParametrosEspacio::default(),
            ganancia_balance_izq: 1.0,
            ganancia_balance_der: 1.0,
        };
        proc.recalcular_ganancias_balance();
        proc
    }

    pub fn actualizar_parametros(&mut self, nuevos: ParametrosEspacio) {
        self.parametros = nuevos;
        self.recalcular_ganancias_balance();
    }

    pub fn obtener_parametros(&self) -> ParametrosEspacio {
        self.parametros
    }

    fn recalcular_ganancias_balance(&mut self) {
        let bal = self.parametros.balance.clamp(-1.0, 1.0);
        // Ley de paneo de potencia constante normalizada a 1.0 en el centro (bal = 0.0)
        let angulo = (bal + 1.0) * FRAC_PI_4;
        let raiz_dos = std::f64::consts::SQRT_2;
        self.ganancia_balance_izq = angulo.cos() * raiz_dos;
        self.ganancia_balance_der = angulo.sin() * raiz_dos;
    }

    #[inline(always)]
    pub fn procesar_par_estereo(&mut self, mut izq: f64, mut der: f64) -> (f64, f64) {
        // 1. Modo Mono o Matriz de Expansion Estereo Mid/Side
        if self.parametros.modo_mono {
            let mono = (izq + der) * 0.5;
            izq = mono;
            der = mono;
        } else if self.parametros.expansion_estereo > 0.001 {
            let exp = self.parametros.expansion_estereo.clamp(0.0, 1.5);
            let medio = (izq + der) * 0.5;
            let lateral = (izq - der) * 0.5 * (1.0 + exp * 1.4);
            // Compensacion sutil de energia para evitar saturacion al abrir el campo estereo
            let compensacion = 1.0 / (1.0 + exp * 0.25);
            izq = (medio + lateral) * compensacion;
            der = (medio - lateral) * compensacion;
        }

        // 2. Balance L/R
        izq *= self.ganancia_balance_izq;
        der *= self.ganancia_balance_der;

        (izq, der)
    }
}

/// Filtro Peine con amortiguacion paso-bajo en el lazo de realimentacion (Moorer Comb Filter).
#[derive(Debug, Clone)]
struct FiltroPeineAmortiguado {
    bufer: Vec<f64>,
    indice: usize,
    realimentacion: f64,
    estado_filtro: f64,
    amortiguacion_1: f64,
    amortiguacion_2: f64,
}

impl FiltroPeineAmortiguado {
    fn nuevo(tamano_muestras: usize) -> Self {
        Self {
            bufer: vec![0.0; tamano_muestras.max(16)],
            indice: 0,
            realimentacion: 0.84,
            estado_filtro: 0.0,
            amortiguacion_1: 0.2,
            amortiguacion_2: 0.8,
        }
    }

    fn configurar(&mut self, realimentacion: f64, amortiguacion: f64) {
        self.realimentacion = realimentacion.clamp(0.0, 0.985);
        self.amortiguacion_1 = amortiguacion.clamp(0.0, 0.99);
        self.amortiguacion_2 = 1.0 - self.amortiguacion_1;
    }

    #[inline(always)]
    fn procesar(&mut self, entrada: f64) -> f64 {
        let salida = self.bufer[self.indice];
        self.estado_filtro = salida * self.amortiguacion_2 + self.estado_filtro * self.amortiguacion_1;
        if self.estado_filtro.abs() < 1e-24 {
            self.estado_filtro = 0.0;
        }

        let nuevo_valor = entrada + self.estado_filtro * self.realimentacion;
        self.bufer[self.indice] = if nuevo_valor.abs() < 1e-24 { 0.0 } else { nuevo_valor };

        self.indice += 1;
        if self.indice >= self.bufer.len() {
            self.indice = 0;
        }

        salida
    }
}

/// Filtro Paso-Todo (All-Pass Filter) de Schroeder para difusion espacial.
#[derive(Debug, Clone)]
struct FiltroPasoTodo {
    bufer: Vec<f64>,
    indice: usize,
    ganancia_difusion: f64,
}

impl FiltroPasoTodo {
    fn nuevo(tamano_muestras: usize) -> Self {
        Self {
            bufer: vec![0.0; tamano_muestras.max(8)],
            indice: 0,
            ganancia_difusion: 0.5,
        }
    }

    #[inline(always)]
    fn procesar(&mut self, entrada: f64) -> f64 {
        let retardada = self.bufer[self.indice];
        let salida = -entrada + retardada;
        let valor_bufer = entrada + retardada * self.ganancia_difusion;
        self.bufer[self.indice] = if valor_bufer.abs() < 1e-24 { 0.0 } else { valor_bufer };

        self.indice += 1;
        if self.indice >= self.bufer.len() {
            self.indice = 0;
        }

        salida
    }
}

/// Procesador de Reverberacion Estereo completo estilo Poweramp Equalizer.
#[derive(Debug, Clone)]
pub struct ProcesadorReverberacionEstereo {
    parametros: ParametrosReverberacion,
    frecuencia_muestreo: f64,
    // Linea de preretardo circular (hasta 250 ms)
    bufer_preretardo_izq: Vec<f64>,
    bufer_preretardo_der: Vec<f64>,
    cursor_preretardo: usize,
    muestras_preretardo: usize,
    // 8 filtros peine por canal
    peines_izq: Vec<FiltroPeineAmortiguado>,
    peines_der: Vec<FiltroPeineAmortiguado>,
    // 4 filtros all-pass en serie por canal
    pasotodo_izq: Vec<FiltroPasoTodo>,
    pasotodo_der: Vec<FiltroPasoTodo>,
    // Filtro tonal en la cola de reverberacion
    filtro_cola: FiltroBiquadEstereo,
}

impl Default for ProcesadorReverberacionEstereo {
    fn default() -> Self {
        Self::nuevo(48000.0)
    }
}

impl ProcesadorReverberacionEstereo {
    pub fn nuevo(frecuencia_muestreo: f64) -> Self {
        let fs = frecuencia_muestreo.max(8000.0);
        let escala_fs = fs / 44100.0;
        let separacion_estereo = (23.0 * escala_fs) as usize;

        // Constantes de afinacion mutua prima de Freeverb/Moorer escaladas a la tasa de muestreo
        let longitudes_peine_base: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        let longitudes_pasotodo_base: [usize; 4] = [556, 441, 341, 225];

        let peines_izq = longitudes_peine_base
            .iter()
            .map(|&l| FiltroPeineAmortiguado::nuevo(((l as f64) * escala_fs) as usize))
            .collect();
        let peines_der = longitudes_peine_base
            .iter()
            .map(|&l| FiltroPeineAmortiguado::nuevo(((l as f64) * escala_fs) as usize + separacion_estereo))
            .collect();

        let pasotodo_izq = longitudes_pasotodo_base
            .iter()
            .map(|&l| FiltroPasoTodo::nuevo(((l as f64) * escala_fs) as usize))
            .collect();
        let pasotodo_der = longitudes_pasotodo_base
            .iter()
            .map(|&l| FiltroPasoTodo::nuevo(((l as f64) * escala_fs) as usize + separacion_estereo))
            .collect();

        let max_preretardo = ((0.250 * fs) as usize).max(64);

        let mut reverb = Self {
            parametros: ParametrosReverberacion::default(),
            frecuencia_muestreo: fs,
            bufer_preretardo_izq: vec![0.0; max_preretardo],
            bufer_preretardo_der: vec![0.0; max_preretardo],
            cursor_preretardo: 0,
            muestras_preretardo: 1,
            peines_izq,
            peines_der,
            pasotodo_izq,
            pasotodo_der,
            filtro_cola: FiltroBiquadEstereo::nuevo(),
        };
        reverb.recalcular_parametros_internos();
        reverb
    }

    pub fn actualizar_parametros(&mut self, nuevos: ParametrosReverberacion) {
        self.parametros = nuevos;
        self.recalcular_parametros_internos();
    }

    pub fn obtener_parametros(&self) -> ParametrosReverberacion {
        self.parametros
    }

    fn recalcular_parametros_internos(&mut self) {
        let realimentacion = 0.70 + self.parametros.tamano.clamp(0.0, 1.0) * 0.27;
        let amortiguacion = self.parametros.amortiguacion.clamp(0.0, 1.0) * 0.65;

        for peine in &mut self.peines_izq {
            peine.configurar(realimentacion, amortiguacion);
        }
        for peine in &mut self.peines_der {
            peine.configurar(realimentacion, amortiguacion);
        }

        let max_pre = self.bufer_preretardo_izq.len().saturating_sub(1).max(1);
        let muestras_deseadas = ((self.parametros.preretardo_ms.clamp(0.0, 200.0) * 0.001)
            * self.frecuencia_muestreo) as usize;
        self.muestras_preretardo = muestras_deseadas.clamp(1, max_pre);

        // Filtro tonal en la cola: < 0.5 atenua agudos (calido), > 0.5 realza aire
        let ganancia_tonal_db = (self.parametros.filtro_tonal.clamp(0.0, 1.0) - 0.5) * 12.0;
        let _ = self.filtro_cola.configurar_parametros(
            4500.0,
            0.707,
            ganancia_tonal_db,
            TipoFiltro::EstanteAgudos,
            self.frecuencia_muestreo,
            ganancia_tonal_db.abs() > 0.1,
        );
    }

    #[inline(always)]
    pub fn procesar_par_estereo(&mut self, muestra_izq: f64, muestra_der: f64) -> (f64, f64) {
        if !self.parametros.activo || self.parametros.mezclado <= 0.001 {
            return (muestra_izq, muestra_der);
        }

        let capacidad_pre = self.bufer_preretardo_izq.len();
        self.bufer_preretardo_izq[self.cursor_preretardo] = muestra_izq;
        self.bufer_preretardo_der[self.cursor_preretardo] = muestra_der;

        let indice_lectura = (self.cursor_preretardo + capacidad_pre - self.muestras_preretardo) % capacidad_pre;
        self.cursor_preretardo = (self.cursor_preretardo + 1) % capacidad_pre;

        // Atenuacion de entrada al tanque de reverberacion para mantener headroom limpio
        let entrada_tanque_izq = self.bufer_preretardo_izq[indice_lectura] * 0.018;
        let entrada_tanque_der = self.bufer_preretardo_der[indice_lectura] * 0.018;

        let mut acum_izq = 0.0;
        let mut acum_der = 0.0;

        for peine in &mut self.peines_izq {
            acum_izq += peine.procesar(entrada_tanque_izq);
        }
        for peine in &mut self.peines_der {
            acum_der += peine.procesar(entrada_tanque_der);
        }

        for pasotodo in &mut self.pasotodo_izq {
            acum_izq = pasotodo.procesar(acum_izq);
        }
        for pasotodo in &mut self.pasotodo_der {
            acum_der = pasotodo.procesar(acum_der);
        }

        let (humedo_izq, humedo_der) = self.filtro_cola.procesar_par_estereo(acum_izq, acum_der);

        let mezcla_humeda = self.parametros.mezclado.clamp(0.0, 1.0);
        let mezcla_seca = 1.0 - mezcla_humeda * 0.45;

        (
            muestra_izq * mezcla_seca + humedo_izq * mezcla_humeda,
            muestra_der * mezcla_seca + humedo_der * mezcla_humeda,
        )
    }
}
