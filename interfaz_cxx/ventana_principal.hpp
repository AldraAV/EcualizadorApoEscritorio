#pragma once

#include "nucleo_dsp/ecualizador_parametrico.hpp"
#include "nucleo_dsp/cargador_peq.hpp"
#include <vector>
#include <string>

namespace aldra::interfaz {

/**
 * @brief Estructura que enlaza la visualización e interacciones de usuario final
 * con cada biquad del motor DSP en C++.
 */
struct BandaVisual {
    int identificador;
    double frecuencia_hz;
    double factor_q;
    float ganancia_db;
    std::string nombre_humano;      // Humanizer: Para usuario común ("Golpe y Ritmo")
    std::string frecuencia_texto;   // Ej. "62 Hz", "1 kHz"
    std::string descripcion_ayuda;  // Tooltip explicativo sin tecnicismos
};

/**
 * @brief Ventana y cabina principal de ecualización nativa C++ al estilo Poweramp.
 * Gestiona el estado de las perillas rápidas (Bass/Treble), las 10 bandas y los presets.
 */
class VentanaPrincipalEcualizador {
public:
    explicit VentanaPrincipalEcualizador(double frecuencia_muestreo_hz = 48000.0);
    ~VentanaPrincipalEcualizador() = default;

    /**
     * @brief Dibuja el tablero de mandos completo en la ventana de Dear ImGui actual.
     */
     void dibujar_cabina_principal() noexcept;

    /**
     * @brief Aplica las ganancias y perillas actuales al motor DSP interno en tiempo real.
     */
    void sincronizar_motor_dsp() noexcept;

    /**
     * @brief Carga y aplica un archivo de preset `.peq-settings` de Poweramp desde disco.
     */
    bool cargar_preset_poweramp_desde_ruta(const std::string& ruta_archivo);

    /**
     * @brief Aplica un preajuste rápido predeterminado en español (ej. "Aplanar Todo", "Potenciar Graves").
     */
    void aplicar_preajuste_rapido(const std::string& nombre_preajuste) noexcept;

    [[nodiscard]] const aldra::audio::EcualizadorParametrico& obtener_motor() const noexcept { return motor_dsp; }
    [[nodiscard]] aldra::audio::EcualizadorParametrico& obtener_motor() noexcept { return motor_dsp; }

private:
    void inicializar_bandas_humanas();
    void dibujar_barra_superior_y_preajustes();
    void dibujar_seccion_perillas_rapidas();
    void dibujar_faders_10_bandas();

    aldra::audio::EcualizadorParametrico motor_dsp;
    std::vector<BandaVisual> lista_bandas_ui;

    // Perillas de acceso rápido al estilo Poweramp
    float ganancia_graves_rapido_db{0.0f};  // Controla filtro estante en 100 Hz
    float ganancia_agudos_rapido_db{0.0f};  // Controla filtro estante en 10 kHz
    float volumen_general_db{0.0f};         // Pre-amplificador maestro
    float factor_q_general{1.414f};         // Anchura global de bandas

    bool mostrar_ayuda_humanizer{true};     // Alternador de descripciones sencillas
    std::string mensaje_estado{"Cabina C++ Poweramp en línea — Listo para ecualizar"};

    // Variables de simulación visual y respuesta reactiva para medidor VU y picos LED
    float medidor_izq_db{-28.0f};
    float medidor_der_db{-29.5f};
    float pico_izq_db{-8.0f};
    float pico_der_db{-9.5f};
};

} // namespace aldra::interfaz
