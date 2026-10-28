#pragma once

#include "imgui.h"

namespace aldra::audio {
    class EcualizadorParametrico; // Declaración adelantada para evitar dependencias cruzadas
}

namespace aldra::interfaz {

/**
 * @brief Dibuja una perilla rotativa 3D de alta ergonomía al estilo Poweramp.
 * Permite control mediante arrastre vertical del ratón o rueda de desplazamiento.
 * 
 * @param etiqueta Identificador único de ImGui y título del control (ej. "Graves Rápido").
 * @param valor Puntero al valor numérico controlado.
 * @param valor_minimo Límite inferior del rango (ej. -12.0f dB).
 * @param valor_maximo Límite superior del rango (ej. +12.0f dB).
 * @param formato_texto Formato de visualización (ej. "%+.1f dB").
 * @param descripcion_humana Explicación sencilla para el usuario final común (humanizer).
 * @param radio_pixeles Radio externo de la perilla en pantalla.
 * @return true Si el usuario modificó el valor en este fotograma.
 */
bool dibujar_perilla_rotativa_poweramp(
    const char* etiqueta,
    float* valor,
    float valor_minimo,
    float valor_maximo,
    const char* formato_texto,
    const char* descripcion_humana,
    float radio_pixeles = 38.0f
) noexcept;

/**
 * @brief Dibuja un fader vertical esqueumórfico para una banda individual del ecualizador.
 * Incluye nombre humano arriba ("Golpe y Ritmo"), valor en dB abajo ("+3.2 dB") y encaje magnético
 * rápido con doble clic sobre el riel para regresar instantáneamente a 0.0 dB.
 * 
 * @param identificador_banda ID único para ImGui.
 * @param nombre_humano Nombre claro para usuario común (ej. "Brillo Cristalino").
 * @param frecuencia_texto Frecuencia en Hz/kHz (ej. "4 kHz").
 * @param ganancia_db Puntero a la ganancia en decibelios de la banda.
 * @param minimo_db Límite inferior (-15.0f).
 * @param maximo_db Límite superior (+15.0f).
 * @param alto_pixeles Altura total del fader vertical en pantalla.
 * @return true Si el valor fue ajustado.
 */
bool dibujar_fader_banda_poweramp(
    const char* identificador_banda,
    const char* nombre_humano,
    const char* frecuencia_texto,
    float* ganancia_db,
    float minimo_db = -15.0f,
    float maximo_db = +15.0f,
    float alto_pixeles = 180.0f
) noexcept;

/**
 * @brief Dibuja el analizador y curva de respuesta espectral ámbar solar desde 20 Hz hasta 20 kHz.
 * Evalúa en tiempo real la respuesta en decibelios de todas las bandas activas de nuestro motor DSP.
 * 
 * @param ecualizador Referencia al motor DSP activo.
 * @param dimensiones Tamaño del panel del gráfico de curva (ancho x alto en píxeles).
 */
void dibujar_curva_espectral_poweramp(
    const aldra::audio::EcualizadorParametrico& ecualizador,
    const ImVec2& dimensiones
) noexcept;

/**
 * @brief Dibuja un medidor de nivel VU estéreo interactivo al estilo Poweramp con picos LED
 * en escala Ámbar Sunlight (Verde -> Ámbar -> Rojo de recorte).
 * 
 * @param nivel_izquierdo_db Nivel instantáneo del canal izquierdo en dB (-60 a 0 dB).
 * @param nivel_derecho_db Nivel instantáneo del canal derecho en dB (-60 a 0 dB).
 * @param pico_izquierdo_db Nivel de pico retenido canal izquierdo.
 * @param pico_derecho_db Nivel de pico retenido canal derecho.
 * @param dimensiones Ancho y alto del medidor en píxeles.
 */
void dibujar_medidor_vu_estereo_poweramp(
    float nivel_izquierdo_db,
    float nivel_derecho_db,
    float pico_izquierdo_db,
    float pico_derecho_db,
    const ImVec2& dimensiones
) noexcept;

} // namespace aldra::interfaz
