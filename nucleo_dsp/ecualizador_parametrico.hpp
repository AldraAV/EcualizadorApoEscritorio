#pragma once

#include "filtro_biquad.hpp"
#include <vector>
#include <memory>
#include <mutex>

namespace aldra::audio {

/**
 * @brief Orquestador del ecualizador paramétrico/gráfico multinivel.
 * Manifiesto de diseño: gestiona múltiples bandas en cascada (para canal izquierdo y derecho)
 * de forma thread-safe y optimizada para llamadas en tiempo real desde el APO.
 */
class EcualizadorParametrico {
public:
    explicit EcualizadorParametrico(double frecuencia_muestreo = 48000.0);
    ~EcualizadorParametrico() = default;

    /**
     * @brief Establece una nueva frecuencia de muestreo para todos los filtros internos.
     */
    void establecer_frecuencia_muestreo(double nueva_frecuencia_muestreo);

    /**
     * @brief Agrega o reemplaza una banda en el motor del ecualizador.
     */
    void configurar_banda(const DatosBanda& banda);

    /**
     * @brief Elimina una banda mediante su identificador único.
     */
    void eliminar_banda(int identificador);

    /**
     * @brief Procesa un búfer estéreo intercalado (L, R, L, R, ...) en tiempo real.
     * @param buffer Puntero al búfer de muestras flotantes de 32 bits (formato estándar APO).
     * @param numero_muestras Cantidad total de muestras individuales en el búfer (ej. frames * 2).
     */
    void procesar_buffer_estereo(float* buffer, size_t numero_muestras) noexcept;

    /**
     * @brief Limpia el estado de los registros de todos los biquads activos para evitar clics al cambiar de pista.
     */
    void reiniciar_historial_filtros() noexcept;

    /**
     * @brief Evalúa la curva de respuesta global en dB para una frecuencia específica.
     */
    [[nodiscard]] double evaluar_respuesta_db(double frecuencia_hz) const;

private:
    double frecuencia_muestreo_actual;
    std::vector<DatosBanda> lista_bandas;
    std::vector<FiltroBiquad> filtros_canal_izquierdo;
    std::vector<FiltroBiquad> filtros_canal_derecho;
    mutable std::mutex mutex_parametros; // Protege la reconfiguración concurrente con el procesamiento de audio
};

} // namespace aldra::audio
