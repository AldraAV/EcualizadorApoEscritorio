#pragma once

#include <stdexcept>
#include <string>
#include <vector>
#include <memory>
#include <cmath>

namespace aldra::audio {

// Excepciones resilientes pre-ejecución del Aldraverso
class ErrorCalculoDsp : public std::runtime_error {
public:
    explicit ErrorCalculoDsp(const std::string& mensaje) 
        : std::runtime_error("Error de Procesamiento DSP: " + mensaje) {}
};

class FrecuenciaFueraDeRango : public ErrorCalculoDsp {
public:
    explicit FrecuenciaFueraDeRango(double frecuencia) 
        : ErrorCalculoDsp("La frecuencia proporcionada (" + std::to_string(frecuencia) + " Hz) está fuera del rango audible o superó la frecuencia de Nyquist.") {}
};

class ParametroAudioInvalido : public ErrorCalculoDsp {
public:
    explicit ParametroAudioInvalido(const std::string& razon) 
        : ErrorCalculoDsp("Parámetro de audio inválido: " + razon) {}
};

// Tipos de filtro según inspección de Poweramp (Band.smali -> type:I)
enum class TipoFiltro {
    PasoAlto = 1,
    PasoBajo = 2,
    PicoParametrico = 3,
    EstanteGraves = 4,
    EstanteAgudos = 5
};

// Estructura que modela una banda individual de ecualizador
struct DatosBanda {
    int identificador;
    double frecuencia_hz;
    double factor_q;
    double ganancia_db;
    TipoFiltro tipo;
    bool activo;
    int canales_afectados; // 0 = Todos, 1 = Izquierdo, 2 = Derecho

    DatosBanda(int id, double freq, double q, double ganancia, TipoFiltro t = TipoFiltro::PicoParametrico, bool encendido = true, int canales = 0)
        : identificador(id), frecuencia_hz(freq), factor_q(q), ganancia_db(ganancia), tipo(t), activo(encendido), canales_afectados(canales) {
        if (frecuencia_hz <= 0.0 || factor_q <= 0.0) {
            throw ParametroAudioInvalido("Frecuencia y factor Q deben ser estrictamente positivos.");
        }
    }
};

} // namespace aldra::audio
