#pragma once

#include "nucleo_dsp.h"
#include <vector>
#include <mutex>

// Forward declaration para evitar incluir miniaudio.h en la cabecera
struct ma_device;

struct DatosEspectro {
    std::vector<float> magnitudes;
    bool actualizados;
};

class MotorAudio {
private:
    ma_device* dispositivo_audio;
    EcualizadorDSP ecualizador;
    
    // Variables para el análisis de espectro (FFT)
    std::vector<float> buffer_fft;
    std::vector<float> ventana_hann;
    int indice_buffer_fft;
    int tamano_fft;
    
    DatosEspectro espectro_actual;
    std::mutex mutex_espectro;

    void procesar_fft();

public:
    MotorAudio();
    ~MotorAudio();

    bool inicializar_dispositivo();
    void iniciar();
    void detener();

    EcualizadorDSP& obtener_ecualizador() { return ecualizador; }
    
    // Callback llamado por miniaudio
    void callback_audio(void* salida, const void* entrada, unsigned int fotogramas);

    // Obtener los datos del espectro rítmico para el Frontend (a 60fps)
    DatosEspectro obtener_datos_espectro();
};
