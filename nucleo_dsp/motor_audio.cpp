#define MINIAUDIO_IMPLEMENTATION
#include "../externos/miniaudio.h"
#include "motor_audio.h"
#include <iostream>
#include <complex>
#include <cmath>
#include <algorithm>

// ==========================================
// Función C-Callback de miniaudio
// ==========================================
void callback_miniaudio(ma_device* dispositivo, void* salida, const void* entrada, ma_uint32 fotogramas) {
    MotorAudio* motor = static_cast<MotorAudio*>(dispositivo->pUserData);
    motor->callback_audio(salida, entrada, fotogramas);
}

// ==========================================
// Utilidad FFT Radix-2 Simple (Cooley-Tukey)
// ==========================================
void ejecutar_fft(std::vector<std::complex<float>>& datos) {
    int n = datos.size();
    if (n <= 1) return;

    // Dividir en pares e impares
    std::vector<std::complex<float>> pares(n / 2), impares(n / 2);
    for (int i = 0; i < n / 2; ++i) {
        pares[i] = datos[i * 2];
        impares[i] = datos[i * 2 + 1];
    }

    // Conquistar (recursividad)
    ejecutar_fft(pares);
    ejecutar_fft(impares);

    // Combinar
    for (int k = 0; k < n / 2; ++k) {
        std::complex<float> t = std::polar(1.0f, -2.0f * (float)std::numbers::pi * k / n) * impares[k];
        datos[k] = pares[k] + t;
        datos[k + n / 2] = pares[k] - t;
    }
}

// ==========================================
// Implementación del Motor de Audio
// ==========================================

MotorAudio::MotorAudio() {
    dispositivo_audio = new ma_device();
    
    // Configuración FFT (Tamano de 1024 o 2048 para buen detalle rítmico)
    tamano_fft = 1024;
    buffer_fft.resize(tamano_fft, 0.0f);
    ventana_hann.resize(tamano_fft, 0.0f);
    indice_buffer_fft = 0;
    
    // Generar ventana de Hann
    for (int i = 0; i < tamano_fft; ++i) {
        ventana_hann[i] = 0.5f * (1.0f - std::cos(2.0f * std::numbers::pi * i / (tamano_fft - 1)));
    }
    
    espectro_actual.magnitudes.resize(tamano_fft / 2, 0.0f);
    espectro_actual.actualizados = false;
}

MotorAudio::~MotorAudio() {
    detener();
    ma_device_uninit(dispositivo_audio);
    delete dispositivo_audio;
}

bool MotorAudio::inicializar_dispositivo() {
    ma_device_config config = ma_device_config_init(ma_device_type_loopback);
    config.capture.format   = ma_format_f32;  // Flotante 32 bits
    config.capture.channels = 2;              // Estéreo
    config.sampleRate       = 48000;          // Tasa estándar
    config.dataCallback     = callback_miniaudio;
    config.pUserData        = this;

    if (ma_device_init(NULL, &config, dispositivo_audio) != MA_SUCCESS) {
        std::cerr << "{\"error\": \"Fallo al inicializar loopback con miniaudio\"}\n";
        return false;
    }
    
    ecualizador.inicializar(dispositivo_audio->sampleRate);
    // Remover cout para que el stdout sea 100% JSON puro
    return true;
}

void MotorAudio::iniciar() {
    ma_device_start(dispositivo_audio);
}

void MotorAudio::detener() {
    ma_device_stop(dispositivo_audio);
}

void MotorAudio::callback_audio(void* salida, const void* entrada, unsigned int fotogramas) {
    // Modo Loopback: 'entrada' contiene el audio del sistema
    if (entrada == nullptr) return;
    
    const float* buffer_entrada = static_cast<const float*>(entrada);
    
    // 2. Acumular muestras para el cálculo del Espectro Rítmico (Mix L+R)
    for (unsigned int i = 0; i < fotogramas; ++i) {
        buffer_fft[indice_buffer_fft] = (buffer_entrada[i*2] + buffer_entrada[i*2 + 1]) * 0.5f;
        indice_buffer_fft++;
        
        if (indice_buffer_fft >= tamano_fft) {
            procesar_fft();
            indice_buffer_fft = 0;
        }
    }
}

void MotorAudio::procesar_fft() {
    std::vector<std::complex<float>> datos_complejos(tamano_fft);
    
    // Aplicar ventana de Hann
    for (int i = 0; i < tamano_fft; ++i) {
        datos_complejos[i] = buffer_fft[i] * ventana_hann[i];
    }
    
    // Calcular FFT
    ejecutar_fft(datos_complejos);
    
    // Calcular magnitudes y actualizar estado protegido por mutex
    std::lock_guard<std::mutex> seguro(mutex_espectro);
    for (int i = 0; i < tamano_fft / 2; ++i) {
        float magnitud = std::abs(datos_complejos[i]);
        // Suavizado temporal y conversión simple logarítmica para visualización rítmica neón
        float db = 20.0f * std::log10(magnitud + 1e-6f);
        float valor_normalizado = std::max(0.0f, (db + 60.0f) / 60.0f); 
        
        // Efecto "caída lenta" (decay)
        espectro_actual.magnitudes[i] = std::max(valor_normalizado, espectro_actual.magnitudes[i] - 0.05f);
    }
    espectro_actual.actualizados = true;
}

DatosEspectro MotorAudio::obtener_datos_espectro() {
    std::lock_guard<std::mutex> seguro(mutex_espectro);
    espectro_actual.actualizados = false;
    return espectro_actual;
}
