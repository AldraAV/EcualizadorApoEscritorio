//! Cadena Maestra de Procesamiento Digital de Senales (DSP) en Tiempo Real.
//! Orquesta en cascada:
//! 1. Preamplificador con suavizado exponencial.
//! 2. Controles de Tono (Estante de Graves y Estante de Agudos).
//! 3. Banco de N Filtros Biquad Parametricos / Graficos en precision de 64 bits.
//! 4. Procesador de Espacio (Balance, Expansion Estereo Mid/Side, Mono).
//! 5. Compresor Dinamico Estereo con filtro Sidechain y rodilla suave.
//! 6. Reverberacion Estereo Schroeder-Moorer / Freeverb.
//! 7. Limitador Maestro Anti-Recorte.
//! 8. Analizador de Espectro FFT Cooley-Tukey de 1024 puntos a 64 barras logarítmicas.

use crate::analizador_espectro::{AnalizadorEspectroTiempoReal, NUMERO_BARRAS_ESPECTRO};
use crate::compresor_dinamico::{CompresorDinamicoEstereo, LimitadorMaestroEstereo};
use crate::filtro_biquad::FiltroBiquadEstereo;
use crate::procesador_espacio_reverb::{ProcesadorEspacioTiempo, ProcesadorReverberacionEstereo};
use crate::tipos_audio::{
    BandaParametrica, ErrorMotorAudio, EstadoCompletoDsp, ParametrosCompresor, ParametrosEspacio,
    ParametrosReverberacion, ParametrosTono, TipoFiltro,
};

/// Cadena Maestra DSP completa con fidelidad 1:1 a Poweramp Equalizer.
#[derive(Debug, Clone)]
pub struct CadenaMaestraDsp {
    estado: EstadoCompletoDsp,
    ganancia_preamp_actual: f64,
    ganancia_preamp_objetivo: f64,
    filtro_tono_graves: FiltroBiquadEstereo,
    filtro_tono_agudos: FiltroBiquadEstereo,
    filtros_bandas: Vec<FiltroBiquadEstereo>,
    procesador_espacio: ProcesadorEspacioTiempo,
    compresor: CompresorDinamicoEstereo,
    reverberacion: ProcesadorReverberacionEstereo,
    limitador: LimitadorMaestroEstereo,
    analizador_espectro: AnalizadorEspectroTiempoReal,
    ultimo_pico_salida_db: f64,
}

impl Default for CadenaMaestraDsp {
    fn default() -> Self {
        Self::nueva(48000.0)
    }
}

impl CadenaMaestraDsp {
    /// Crea e inicializa toda la cadena DSP a la frecuencia de muestreo indicada.
    pub fn nueva(frecuencia_muestreo_hz: f64) -> Self {
        let fs = frecuencia_muestreo_hz.max(8000.0);
        let mut estado = EstadoCompletoDsp::default();
        estado.frecuencia_muestreo_hz = fs;

        let mut cadena = Self {
            estado: estado.clone(),
            ganancia_preamp_actual: 1.0,
            ganancia_preamp_objetivo: 1.0,
            filtro_tono_graves: FiltroBiquadEstereo::nuevo(),
            filtro_tono_agudos: FiltroBiquadEstereo::nuevo(),
            filtros_bandas: Vec::with_capacity(32),
            procesador_espacio: ProcesadorEspacioTiempo::nuevo(fs),
            compresor: CompresorDinamicoEstereo::nuevo(fs),
            reverberacion: ProcesadorReverberacionEstereo::nuevo(fs),
            limitador: LimitadorMaestroEstereo::nuevo(fs),
            analizador_espectro: AnalizadorEspectroTiempoReal::default(),
            ultimo_pico_salida_db: -96.0,
        };

        let _ = cadena.sincronizar_filtros_internos();
        cadena
    }

    /// Devuelve una referencia inmutable al estado actual de la cadena DSP.
    pub fn obtener_estado(&self) -> &EstadoCompletoDsp {
        &self.estado
    }

    /// Devuelve una copia de las 64 barras actuales del analizador de espectro [0.0 .. 1.0].
    pub fn obtener_barras_espectro(&self) -> [f32; NUMERO_BARRAS_ESPECTRO] {
        self.analizador_espectro.obtener_barras()
    }

    /// Devuelve la reduccion de ganancia instantanea del compresor en dB.
    pub fn obtener_reduccion_compresor_db(&self) -> f64 {
        self.compresor.obtener_reduccion_db()
    }

    /// Devuelve el ultimo nivel pico de salida medido en dBFS.
    pub fn obtener_pico_salida_db(&self) -> f64 {
        self.ultimo_pico_salida_db
    }

    /// Reemplaza el estado completo (por ejemplo al cargar un preset JSON o AutoEQ) y recalcula coeficientes.
    pub fn aplicar_estado(&mut self, mut nuevo_estado: EstadoCompletoDsp) -> Result<(), ErrorMotorAudio> {
        if nuevo_estado.frecuencia_muestreo_hz <= 0.0 {
            nuevo_estado.frecuencia_muestreo_hz = self.estado.frecuencia_muestreo_hz;
        }
        self.estado = nuevo_estado;
        self.sincronizar_filtros_internos()
    }

    /// Activa o desactiva globalmente el ecualizador.
    pub fn establecer_ecualizador_activo(&mut self, activo: bool) {
        self.estado.ecualizador_activo = activo;
    }

    /// Establece la ganancia del preamplificador maestro en dB (-24.0 a +24.0 dB).
    pub fn establecer_preamp_db(&mut self, ganancia_db: f64) {
        self.estado.ganancia_preamp_db = ganancia_db.clamp(-24.0, 24.0);
        self.ganancia_preamp_objetivo = 10.0_f64.powf(self.estado.ganancia_preamp_db / 20.0);
    }

    /// Activa o desactiva el limitador maestro de salida.
    pub fn establecer_limitador_activo(&mut self, activo: bool) {
        self.estado.limitador_activo = activo;
    }

    /// Actualiza rapidamente la ganancia en dB de una banda (indice 1-basado).
    pub fn actualizar_ganancia_banda(
        &mut self,
        identificador_uno_basado: usize,
        nueva_ganancia_db: f64,
    ) -> Result<(), ErrorMotorAudio> {
        let indice = identificador_uno_basado.saturating_sub(1);
        if indice >= self.estado.bandas.len() {
            return Err(ErrorMotorAudio::FormatoPresetInvalido(format!(
                "Indice de banda fuera de rango: {} (total: {})",
                identificador_uno_basado,
                self.estado.bandas.len()
            )));
        }

        self.estado.bandas[indice].ganancia_db = nueva_ganancia_db.clamp(-24.0, 24.0);
        let banda_clon = self.estado.bandas[indice].clone();
        if indice < self.filtros_bandas.len() {
            self.filtros_bandas[indice].configurar_banda(&banda_clon, self.estado.frecuencia_muestreo_hz)?;
        }
        Ok(())
    }

    /// Actualiza todos los parametros de una banda existente (indice 1-basado).
    pub fn actualizar_parametros_banda(
        &mut self,
        identificador_uno_basado: usize,
        frecuencia_hz: f64,
        factor_q: f64,
        ganancia_db: f64,
        tipo: TipoFiltro,
        activo: bool,
    ) -> Result<(), ErrorMotorAudio> {
        let indice = identificador_uno_basado.saturating_sub(1);
        if indice >= self.estado.bandas.len() {
            return Err(ErrorMotorAudio::FormatoPresetInvalido(format!(
                "Indice de banda fuera de rango: {}",
                identificador_uno_basado
            )));
        }

        let banda = &mut self.estado.bandas[indice];
        banda.frecuencia_hz = frecuencia_hz.clamp(16.0, 22000.0);
        banda.factor_q = factor_q.clamp(0.1, 25.0);
        banda.ganancia_db = ganancia_db.clamp(-24.0, 24.0);
        banda.tipo = tipo;
        banda.activo = activo;

        let banda_clon = banda.clone();
        if indice < self.filtros_bandas.len() {
            self.filtros_bandas[indice].configurar_banda(&banda_clon, self.estado.frecuencia_muestreo_hz)?;
        }
        Ok(())
    }

    /// Agrega una nueva banda parametrica al banco de filtros (maximo 32 bandas).
    pub fn agregar_banda(&mut self, mut nueva_banda: BandaParametrica) -> Result<(), ErrorMotorAudio> {
        if self.estado.bandas.len() >= 32 {
            return Err(ErrorMotorAudio::FormatoPresetInvalido(String::from(
                "Se alcanzo el limite maximo de 32 bandas parametricas",
            )));
        }
        nueva_banda.identificador = self.estado.bandas.len() + 1;
        let mut filtro = FiltroBiquadEstereo::nuevo();
        filtro.configurar_banda(&nueva_banda, self.estado.frecuencia_muestreo_hz)?;
        self.estado.bandas.push(nueva_banda);
        self.filtros_bandas.push(filtro);
        Ok(())
    }

    /// Elimina una banda por su identificador 1-basado y renumera las restantes.
    pub fn eliminar_banda(&mut self, identificador_uno_basado: usize) -> Result<(), ErrorMotorAudio> {
        if self.estado.bandas.len() <= 1 {
            return Err(ErrorMotorAudio::FormatoPresetInvalido(String::from(
                "El ecualizador debe conservar al menos 1 banda",
            )));
        }
        let indice = identificador_uno_basado.saturating_sub(1);
        if indice >= self.estado.bandas.len() {
            return Err(ErrorMotorAudio::FormatoPresetInvalido(format!(
                "Indice de banda invalido para eliminar: {}",
                identificador_uno_basado
            )));
        }

        self.estado.bandas.remove(indice);
        if indice < self.filtros_bandas.len() {
            self.filtros_bandas.remove(indice);
        }
        for (i, banda) in self.estado.bandas.iter_mut().enumerate() {
            banda.identificador = i + 1;
        }
        Ok(())
    }

    /// Actualiza los controles de Tono (Graves y Agudos).
    pub fn actualizar_tono(&mut self, nuevo_tono: ParametrosTono) -> Result<(), ErrorMotorAudio> {
        self.estado.tono = nuevo_tono;
        self.sincronizar_filtros_tono()
    }

    /// Actualiza los controles de Espacio y Tiempo.
    pub fn actualizar_espacio(&mut self, nuevo_espacio: ParametrosEspacio) {
        self.estado.espacio = nuevo_espacio;
        self.procesador_espacio.actualizar_parametros(nuevo_espacio);
    }

    /// Actualiza los controles del Compresor Dinamico.
    pub fn actualizar_compresor(&mut self, nuevo_compresor: ParametrosCompresor) {
        self.estado.compresor = nuevo_compresor;
        self.compresor.actualizar_parametros(nuevo_compresor);
    }

    /// Actualiza los controles de Reverberacion Estereo.
    pub fn actualizar_reverberacion(&mut self, nueva_reverb: ParametrosReverberacion) {
        self.estado.reverberacion = nueva_reverb;
        self.reverberacion.actualizar_parametros(nueva_reverb);
    }

    fn sincronizar_filtros_tono(&mut self) -> Result<(), ErrorMotorAudio> {
        let fs = self.estado.frecuencia_muestreo_hz;
        let t = self.estado.tono;

        let graves_activo = t.tono_activo && t.graves_ganancia_db.abs() > 0.005;
        self.filtro_tono_graves.configurar_parametros(
            t.graves_frecuencia_hz.clamp(20.0, 500.0),
            t.graves_factor_q.clamp(0.1, 10.0),
            t.graves_ganancia_db.clamp(-24.0, 24.0),
            TipoFiltro::EstanteGraves,
            fs,
            graves_activo,
        )?;

        let agudos_activo = t.tono_activo && t.agudos_ganancia_db.abs() > 0.005;
        self.filtro_tono_agudos.configurar_parametros(
            t.agudos_frecuencia_hz.clamp(1000.0, fs * 0.49),
            t.agudos_factor_q.clamp(0.1, 10.0),
            t.agudos_ganancia_db.clamp(-24.0, 24.0),
            TipoFiltro::EstanteAgudos,
            fs,
            agudos_activo,
        )?;

        Ok(())
    }

    fn sincronizar_filtros_internos(&mut self) -> Result<(), ErrorMotorAudio> {
        let fs = self.estado.frecuencia_muestreo_hz;
        self.ganancia_preamp_objetivo = 10.0_f64.powf(self.estado.ganancia_preamp_db / 20.0);

        self.sincronizar_filtros_tono()?;

        if self.filtros_bandas.len() != self.estado.bandas.len() {
            self.filtros_bandas
                .resize_with(self.estado.bandas.len(), FiltroBiquadEstereo::nuevo);
        }

        for (filtro, banda) in self.filtros_bandas.iter_mut().zip(self.estado.bandas.iter()) {
            filtro.configurar_banda(banda, fs)?;
        }

        self.procesador_espacio.actualizar_parametros(self.estado.espacio);
        self.compresor.establecer_frecuencia_muestreo(fs);
        self.compresor.actualizar_parametros(self.estado.compresor);
        self.reverberacion.actualizar_parametros(self.estado.reverberacion);

        Ok(())
    }

    /// Procesa un par estereo en punto flotante de 32 bits ejecutando toda la cadena en 64 bits.
    #[inline(always)]
    pub fn procesar_par_estereo(&mut self, entrada_izq: f32, entrada_der: f32) -> (f32, f32) {
        if !self.estado.ecualizador_activo {
            self.analizador_espectro
                .alimentar_muestra_estereo(entrada_izq, entrada_der);
            return (entrada_izq, entrada_der);
        }

        // 1. Preamplificador con interpolacion suave
        self.ganancia_preamp_actual +=
            (self.ganancia_preamp_objetivo - self.ganancia_preamp_actual) * 0.008;
        let mut izq = (entrada_izq as f64) * self.ganancia_preamp_actual;
        let mut der = (entrada_der as f64) * self.ganancia_preamp_actual;

        // 2. Filtros de Tono (Graves y Agudos)
        if self.estado.tono.tono_activo {
            (izq, der) = self.filtro_tono_graves.procesar_par_estereo(izq, der);
            (izq, der) = self.filtro_tono_agudos.procesar_par_estereo(izq, der);
        }

        // 3. Cascada de N bandas Biquad parametricas / graficas
        for filtro in &mut self.filtros_bandas {
            (izq, der) = filtro.procesar_par_estereo(izq, der);
        }

        // 4. Procesador de Espacio (Balance, Expansion Mid/Side, Mono)
        (izq, der) = self.procesador_espacio.procesar_par_estereo(izq, der);

        // 5. Compresor Dinamico Estereo con filtro Sidechain
        (izq, der) = self.compresor.procesar_par_estereo(izq, der);

        // 6. Procesador de Reverberacion Estereo
        (izq, der) = self.reverberacion.procesar_par_estereo(izq, der);

        // 7. Limitador Maestro Anti-Clipping
        (izq, der) = self
            .limitador
            .procesar_par_estereo(izq, der, self.estado.limitador_activo);

        let salida_izq = izq as f32;
        let salida_der = der as f32;

        // 8. Alimentar analizador de espectro FFT en tiempo real
        self.analizador_espectro
            .alimentar_muestra_estereo(salida_izq, salida_der);

        let pico = salida_izq.abs().max(salida_der.abs()).max(1e-6) as f64;
        self.ultimo_pico_salida_db = 20.0 * pico.log10();

        (salida_izq, salida_der)
    }

    /// Procesa un bloque completo de muestras estereo entrelazadas `[L0, R0, L1, R1, ...]`.
    pub fn procesar_bloque_entrelazado(&mut self, bufer_entrelazado: &mut [f32]) {
        for par in bufer_entrelazado.chunks_exact_mut(2) {
            let (s_izq, s_der) = self.procesar_par_estereo(par[0], par[1]);
            par[0] = s_izq;
            par[1] = s_der;
        }
    }

    /// Evalua la curva de respuesta en frecuencia combinada (en dB) sobre una lista de frecuencias
    /// usando la ecuacion exacta de `Coefs.smali -> В()` de Poweramp Equalizer.
    pub fn evaluar_curva_respuesta_db(&self, frecuencias_hz: &[f64]) -> Vec<f64> {
        let fs = self.estado.frecuencia_muestreo_hz;
        let mut curva_db = Vec::with_capacity(frecuencias_hz.len());

        for &frec in frecuencias_hz {
            if !self.estado.ecualizador_activo {
                curva_db.push(0.0);
                continue;
            }

            let mut magnitud_acumulada = 1.0_f64;

            if self.estado.tono.tono_activo {
                magnitud_acumulada *= self.filtro_tono_graves.evaluar_magnitud(frec, fs);
                magnitud_acumulada *= self.filtro_tono_agudos.evaluar_magnitud(frec, fs);
            }

            for filtro in &self.filtros_bandas {
                magnitud_acumulada *= filtro.evaluar_magnitud(frec, fs);
            }

            let ganancia_filtros_db = 20.0 * magnitud_acumulada.max(1e-6).log10();
            curva_db.push(self.estado.ganancia_preamp_db + ganancia_filtros_db);
        }

        curva_db
    }
}
