#pragma once

#include "tipos_audio.hpp"
#include <array>

namespace aldra::audio {

/**
 * @brief Filtro IIR de segundo orden (Biquad) implementado con el modelo matemático de Poweramp.
 * 
 * Utiliza precisión doble (`double`) en los coeficientes y buffers internos para prevenir
 * ruido de truncamiento e inestabilidad en las bajas frecuencias, siguiendo el descubrimiento
 * en `Coefs.smali -> В()`.
 */
class FiltroBiquad {
public:
    FiltroBiquad();
    ~FiltroBiquad() = default;

    /**
     * @brief Configura o actualiza los coeficientes del biquad usando la especificación de banda de Poweramp.
     * @param banda Datos descriptivos de la banda (Frecuencia, Q, Ganancia, Tipo).
     * @param frecuencia_muestreo Frecuencia de muestreo actual del sistema en Hz (ej. 48000.0).
     * @throws FrecuenciaFueraDeRango Si la frecuencia supera la mitad de la frecuencia de muestreo (Nyquist).
     */
    void configurar_banda(const DatosBanda& banda, double frecuencia_muestreo);

    /**
     * @brief Procesa una única muestra de audio usando la topología Direct Form II Transposed.
     * @param muestra_entrada Muestra cruda de audio en punto flotante (64-bit o 32-bit promovido).
     * @return Muestra filtrada en punto flotante 64-bit.
     */
    [[nodiscard]] double procesar_muestra(double muestra_entrada) noexcept;

    /**
     * @brief Resetea el historial de memoria de los registros de retardo ($z^{-1}, z^{-2}$).
     */
    void reiniciar_memoria() noexcept;

    /**
     * @brief Evalúa la magnitud o respuesta en frecuencia $|H(e^{j\omega})|$ para una frecuencia en particular.
     * Basado en la fórmula pura del método `В(DDDDDDD)D` de `Coefs.smali`.
     * @param frecuencia_hz Frecuencia en la cual evaluar la ganancia del filtro.
     * @param frecuencia_muestreo Frecuencia de muestreo actual del sistema.
     * @return Magnitud lineal de respuesta (para convertir a dB: 20 * log10(magnitud)).
     */
    [[nodiscard]] double evaluar_magnitud(double frecuencia_hz, double frecuencia_muestreo) const;

private:
    // Coeficientes normalizados del filtro (a0 = 1.0)
    double coeficiente_b0;
    double coeficiente_b1;
    double coeficiente_b2;
    double coeficiente_a1;
    double coeficiente_a2;

    // Registros de estado de retardo (Direct Form II Transposed para máxima precisión numérica)
    double registro_estado_1;
    double registro_estado_2;

    bool esta_activo;
};

} // namespace aldra::audio
