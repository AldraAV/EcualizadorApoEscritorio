//! Analizador de Espectro FFT Cooley-Tukey Radix-2 iterativo in-place (Cero Asignaciones)
//! con ventana de Hann precalculada y suavizado temporal inspirado en `milk_main.lua`
//! de Poweramp Equalizer (`mix1 = (smoothing * 0.97)^0.5`).

use std::f32::consts::PI;

/// Tamano fijo de ventana FFT (potencia de 2) para resolucion rítmica a 60 FPS sin latencia.
pub const TAMANO_FFT: usize = 1024;
pub const NUMERO_BARRAS_ESPECTRO: usize = 64;

/// Analizador de espectro en tiempo real libre de asignaciones en el hilo de audio.
#[derive(Debug, Clone)]
pub struct AnalizadorEspectroTiempoReal {
    bufer_anillo_entrada: [f32; TAMANO_FFT],
    ventana_hann: [f32; TAMANO_FFT],
    indices_bit_inverso: [usize; TAMANO_FFT],
    trabajo_real: [f32; TAMANO_FFT],
    trabajo_imag: [f32; TAMANO_FFT],
    magnitudes_suavizadas: [f32; NUMERO_BARRAS_ESPECTRO],
    cursor_muestras: usize,
    muestras_desde_ultima_fft: usize,
    paso_salto_fft: usize,
    factor_mezcla_1: f32,
    factor_mezcla_2: f32,
}

impl Default for AnalizadorEspectroTiempoReal {
    fn default() -> Self {
        Self::nuevo(0.72)
    }
}

impl AnalizadorEspectroTiempoReal {
    /// Crea el analizador precalculando la ventana de Hann y la tabla de permutacion bit-reversal.
    pub fn nuevo(suavizado: f32) -> Self {
        let mut ventana_hann = [0.0_f32; TAMANO_FFT];
        let mut indices_bit_inverso = [0_usize; TAMANO_FFT];
        let bits = TAMANO_FFT.trailing_zeros();

        for i in 0..TAMANO_FFT {
            ventana_hann[i] = 0.5 * (1.0 - (2.0 * PI * (i as f32) / ((TAMANO_FFT - 1) as f32)).cos());
            indices_bit_inverso[i] = i.reverse_bits() >> (usize::BITS - bits);
        }

        // Formula exacta de suavizado de `assets/milk/lua/milk_main.lua` linea 267:
        // local mix1 = math.pow(wave.smoothing * .97, 0.5)
        let suavizado_acotado = suavizado.clamp(0.0, 0.99);
        let factor_mezcla_1 = (suavizado_acotado * 0.97).sqrt();
        let factor_mezcla_2 = 1.0 - factor_mezcla_1;

        Self {
            bufer_anillo_entrada: [0.0; TAMANO_FFT],
            ventana_hann,
            indices_bit_inverso,
            trabajo_real: [0.0; TAMANO_FFT],
            trabajo_imag: [0.0; TAMANO_FFT],
            magnitudes_suavizadas: [0.0; NUMERO_BARRAS_ESPECTRO],
            cursor_muestras: 0,
            muestras_desde_ultima_fft: 0,
            // Ejecutar FFT cada 512 muestras (solapamiento del 50% a 48kHz -> ~93 Hz de refresco)
            paso_salto_fft: TAMANO_FFT / 2,
            factor_mezcla_1,
            factor_mezcla_2,
        }
    }

    /// Alimenta un par estereo ya procesado por la cadena DSP.
    /// Devuelve `true` cuando un nuevo cuadro de espectro de 64 barras esta listo.
    #[inline(always)]
    pub fn alimentar_muestra_estereo(&mut self, izq: f32, der: f32) -> bool {
        self.bufer_anillo_entrada[self.cursor_muestras] = (izq + der) * 0.5;
        self.cursor_muestras = (self.cursor_muestras + 1) & (TAMANO_FFT - 1);
        self.muestras_desde_ultima_fft += 1;

        if self.muestras_desde_ultima_fft >= self.paso_salto_fft {
            self.muestras_desde_ultima_fft = 0;
            self.ejecutar_fft_en_sitio();
            return true;
        }
        false
    }

    /// Ejecuta la FFT Radix-2 iterativa en sitio sin tocar el heap.
    fn ejecutar_fft_en_sitio(&mut self) {
        // 1. Copiar desde el bufer circular aplicando ventana de Hann y permutacion bit-inverso
        for i in 0..TAMANO_FFT {
            let idx_origen = (self.cursor_muestras + i) & (TAMANO_FFT - 1);
            let idx_destino = self.indices_bit_inverso[i];
            self.trabajo_real[idx_destino] = self.bufer_anillo_entrada[idx_origen] * self.ventana_hann[i];
            self.trabajo_imag[idx_destino] = 0.0;
        }

        // 2. Mariposas iterativas de Cooley-Tukey por etapas (2, 4, 8, ..., 1024)
        let mut longitud = 2;
        while longitud <= TAMANO_FFT {
            let mitad = longitud / 2;
            let angulo_paso = -2.0 * PI / (longitud as f32);
            let w_paso_real = angulo_paso.cos();
            let w_paso_imag = angulo_paso.sin();

            let mut inicio_bloque = 0;
            while inicio_bloque < TAMANO_FFT {
                let mut w_real = 1.0_f32;
                let mut w_imag = 0.0_f32;

                for j in 0..mitad {
                    let i_par = inicio_bloque + j;
                    let i_impar = i_par + mitad;

                    let t_real = w_real * self.trabajo_real[i_impar] - w_imag * self.trabajo_imag[i_impar];
                    let t_imag = w_real * self.trabajo_imag[i_impar] + w_imag * self.trabajo_real[i_impar];

                    self.trabajo_real[i_impar] = self.trabajo_real[i_par] - t_real;
                    self.trabajo_imag[i_impar] = self.trabajo_imag[i_par] - t_imag;
                    self.trabajo_real[i_par] += t_real;
                    self.trabajo_imag[i_par] += t_imag;

                    let siguiente_w_real = w_real * w_paso_real - w_imag * w_paso_imag;
                    w_imag = w_real * w_paso_imag + w_imag * w_paso_real;
                    w_real = siguiente_w_real;
                }

                inicio_bloque += longitud;
            }
            longitud <<= 1;
        }

        // 3. Agrupar bins de frecuencia en 64 barras logarítmicas y aplicar suavizado MilkDrop/Poweramp
        let bins_utiles = TAMANO_FFT / 2;
        for barra in 0..NUMERO_BARRAS_ESPECTRO {
            let prop_inicio = (barra as f32) / (NUMERO_BARRAS_ESPECTRO as f32);
            let prop_fin = ((barra + 1) as f32) / (NUMERO_BARRAS_ESPECTRO as f32);

            // Escala exponencial desde bin 1 (~46 Hz) hasta bin 511 (~24 kHz)
            let bin_ini = ((bins_utiles as f32).powf(prop_inicio) as usize).clamp(1, bins_utiles - 1);
            let bin_fin = ((bins_utiles as f32).powf(prop_fin) as usize).clamp(bin_ini + 1, bins_utiles);

            let mut suma_mag = 0.0_f32;
            for b in bin_ini..bin_fin {
                let re = self.trabajo_real[b];
                let im = self.trabajo_imag[b];
                suma_mag += (re * re + im * im).sqrt();
            }
            let mag_promedio = suma_mag / ((bin_fin - bin_ini) as f32);

            // Normalizar a escala dBFS perceptual [0.0 .. 1.0] con puerta de silencio a -48 dBFS
            let db = 20.0 * (mag_promedio / (TAMANO_FFT as f32 * 0.25) + 1e-6).log10();
            let normalizado = if db < -48.0 {
                0.0
            } else {
                ((db + 48.0) / 48.0).clamp(0.0, 1.0)
            };

            let previo = self.magnitudes_suavizadas[barra];
            self.magnitudes_suavizadas[barra] =
                normalizado * self.factor_mezcla_2 + previo * self.factor_mezcla_1;
        }
    }

    /// Devuelve una copia instantanea del arreglo de 64 barras normalizadas [0.0 .. 1.0].
    #[inline]
    pub fn obtener_barras(&self) -> [f32; NUMERO_BARRAS_ESPECTRO] {
        self.magnitudes_suavizadas
    }
}
