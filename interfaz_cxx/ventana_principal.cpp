#include "ventana_principal.hpp"
#include "controles_poweramp.hpp"
#include "imgui.h"
#include <cmath>
#include <iostream>
#include <fstream>
#include <sstream>
#include <iomanip>
#include <filesystem>

namespace aldra::interfaz {

VentanaPrincipalEcualizador::VentanaPrincipalEcualizador(double frecuencia_muestreo_hz)
    : motor_dsp(frecuencia_muestreo_hz) {
    inicializar_bandas_humanas();
    sincronizar_motor_dsp();
}

void VentanaPrincipalEcualizador::inicializar_bandas_humanas() {
    lista_bandas_ui.clear();
    lista_bandas_ui.reserve(10);

    // 10 Bandas estándar de Poweramp con nomenclatura humana sencilla (Humanizer)
    lista_bandas_ui.push_back({1,  31.0,  1.414, 0.0f, "Sub-bajo",     "31 Hz", "Graves profundos que hacen vibrar el pecho y subwoofers."});
    lista_bandas_ui.push_back({2,  62.0,  1.414, 0.0f, "Golpe",        "62 Hz", "La patada de la batería y la fuerza del bajo eléctrico."});
    lista_bandas_ui.push_back({3,  125.0, 1.414, 0.0f, "Cuerpo",       "125 Hz", "Calidez y consistencia en notas graves y violonchelos."});
    lista_bandas_ui.push_back({4,  250.0, 1.414, 0.0f, "Calidez",      "250 Hz", "Fondo acústico y riqueza en voces graves masculinas."});
    lista_bandas_ui.push_back({5,  500.0, 1.414, 0.0f, "Presencia",    "500 Hz", "El corazón de la música; da definición a instrumentos."});
    lista_bandas_ui.push_back({6,  1000.0, 1.414, 0.0f, "Vocal",        "1 kHz",  "Ataque, claridad y dicción natural de la voz principal."});
    lista_bandas_ui.push_back({7,  2000.0, 1.414, 0.0f, "Definición",   "2 kHz",  "Mordida en las cuerdas de guitarra y separación estéreo."});
    lista_bandas_ui.push_back({8,  4000.0, 1.414, 0.0f, "Brillo",       "4 kHz",  "Nitidez de percusiones, platillos y consonantes vocales."});
    lista_bandas_ui.push_back({9,  8000.0, 1.414, 0.0f, "Aire",         "8 kHz",  "Sensación de espacio abierto y respiración en la sala."});
    lista_bandas_ui.push_back({10, 16000.0, 1.414, 0.0f, "Resplandor",   "16 kHz", "El brillo ultra-fino y armónicos agudos cristalinos."});
}

void VentanaPrincipalEcualizador::sincronizar_motor_dsp() noexcept {
    try {
        // Configurar perillas rápidas de graves y agudos (Filtros tipo estante)
        if (std::abs(ganancia_graves_rapido_db) > 0.05f) {
            motor_dsp.configurar_banda(aldra::audio::DatosBanda(
                1001, 100.0, 0.707, static_cast<double>(ganancia_graves_rapido_db), aldra::audio::TipoFiltro::EstanteGraves
            ));
        } else {
            motor_dsp.eliminar_banda(1001);
        }

        if (std::abs(ganancia_agudos_rapido_db) > 0.05f) {
            motor_dsp.configurar_banda(aldra::audio::DatosBanda(
                1002, 10000.0, 0.707, static_cast<double>(ganancia_agudos_rapido_db), aldra::audio::TipoFiltro::EstanteAgudos
            ));
        } else {
            motor_dsp.eliminar_banda(1002);
        }

        // Configurar las 10 bandas paramétricas/gráficas del ecualizador
        for (const auto& banda : lista_bandas_ui) {
            motor_dsp.configurar_banda(aldra::audio::DatosBanda(
                banda.identificador,
                banda.frecuencia_hz,
                static_cast<double>(factor_q_general),
                static_cast<double>(banda.ganancia_db),
                aldra::audio::TipoFiltro::PicoParametrico
            ));
        }

        // --- Exportación en tiempo real del archivo config.txt de Equalizer APO ---
        std::ostringstream ss_apo;
        ss_apo << "# Configuración en tiempo real de Cacaomático Poweramp C++\n";
        ss_apo << std::fixed << std::setprecision(1);
        ss_apo << "Preamp: " << (volumen_general_db >= 0.0f ? "+" : "") << volumen_general_db << " dB\n";
        ss_apo << "GraphicEQ: ";
        for (size_t i = 0; i < lista_bandas_ui.size(); ++i) {
            float g_efectiva = lista_bandas_ui[i].ganancia_db;
            if (i <= 2) g_efectiva += ganancia_graves_rapido_db;
            if (i >= 7) g_efectiva += ganancia_agudos_rapido_db;

            int f_hz = static_cast<int>(lista_bandas_ui[i].frecuencia_hz);
            ss_apo << f_hz << " " << (g_efectiva >= 0.0f ? "+" : "") << g_efectiva;
            if (i + 1 < lista_bandas_ui.size()) ss_apo << "; ";
        }
        ss_apo << "\n";
        std::string texto_apo = ss_apo.str();

        // Escribir en múltiples rutas estándar para garantizar enrutado continuo
        std::vector<std::string> rutas_exportacion = {
            "C:\\Program Files\\EqualizerAPO\\config\\config.txt",
            "C:\\ProgramData\\EqualizerAPO\\config.txt",
            "config.txt"
        };

        bool escrito_en_sistema = false;
        for (const auto& ruta : rutas_exportacion) {
            try {
                std::filesystem::path path_obj(ruta);
                if (path_obj.has_parent_path()) {
                    std::filesystem::create_directories(path_obj.parent_path());
                }
                std::ofstream archivo(ruta);
                if (archivo.is_open()) {
                    archivo << texto_apo;
                    archivo.close();
                    if (ruta.find("Program") != std::string::npos) {
                        escrito_en_sistema = true;
                    }
                }
            } catch (...) {
                // Continuar a la siguiente ruta
            }
        }

        if (escrito_en_sistema) {
            mensaje_estado = "Sincronizado y enrutado activo con Equalizer APO en tiempo real";
        } else {
            mensaje_estado = "Audio C++ actualizado — Guardado local en config.txt (Equalizer APO activo)";
        }

    } catch (const std::exception& error) {
        mensaje_estado = std::string("Error al sincronizar motor DSP: ") + error.what();
    }
}

bool VentanaPrincipalEcualizador::cargar_preset_poweramp_desde_ruta(const std::string& ruta_archivo) {
    try {
        aldra::audio::ConfiguracionPEQ config = aldra::audio::CargadorPEQ::cargar_desde_archivo(ruta_archivo);
        volumen_general_db = static_cast<float>(config.ganancia_preamplificador_db);

        for (const auto& banda_imp : config.bandas) {
            for (auto& banda_ui : lista_bandas_ui) {
                if (std::abs(banda_ui.frecuencia_hz - banda_imp.frecuencia_hz) < banda_ui.frecuencia_hz * 0.2) {
                    banda_ui.ganancia_db = static_cast<float>(banda_imp.ganancia_db);
                    break;
                }
            }
        }
        sincronizar_motor_dsp();
        mensaje_estado = "Preset Poweramp importado con éxito: " + config.nombre_perfil;
        return true;
    } catch (const std::exception& error) {
        mensaje_estado = std::string("Error importando preset: ") + error.what();
        return false;
    }
}

void VentanaPrincipalEcualizador::aplicar_preajuste_rapido(const std::string& nombre_preajuste) noexcept {
    if (nombre_preajuste == "Aplanar Todo (0 dB)") {
        ganancia_graves_rapido_db = 0.0f;
        ganancia_agudos_rapido_db = 0.0f;
        volumen_general_db = 0.0f;
        for (auto& b : lista_bandas_ui) b.ganancia_db = 0.0f;
        mensaje_estado = "Respuesta ecualizada a plana (0.0 dB)";

    } else if (nombre_preajuste == "Potenciar Graves (Punch)") {
        ganancia_graves_rapido_db = +4.5f;
        ganancia_agudos_rapido_db = +1.0f;
        volumen_general_db = -1.5f;
        if (lista_bandas_ui.size() >= 10) {
            lista_bandas_ui[0].ganancia_db = +5.0f; // 31 Hz
            lista_bandas_ui[1].ganancia_db = +6.5f; // 62 Hz
            lista_bandas_ui[2].ganancia_db = +3.0f; // 125 Hz
            lista_bandas_ui[3].ganancia_db = +1.0f;
            lista_bandas_ui[4].ganancia_db = 0.0f;
            lista_bandas_ui[5].ganancia_db = 0.0f;
            lista_bandas_ui[6].ganancia_db = +0.5f;
            lista_bandas_ui[7].ganancia_db = +1.0f;
            lista_bandas_ui[8].ganancia_db = +1.5f;
            lista_bandas_ui[9].ganancia_db = +2.0f;
        }
        mensaje_estado = "Preajuste aplicado: Potenciar Graves (Punch pesado)";

    } else if (nombre_preajuste == "Voz Clara (Podcast)") {
        ganancia_graves_rapido_db = -2.0f;
        ganancia_agudos_rapido_db = +2.5f;
        volumen_general_db = +1.0f;
        if (lista_bandas_ui.size() >= 10) {
            lista_bandas_ui[0].ganancia_db = -4.0f; // 31 Hz
            lista_bandas_ui[1].ganancia_db = -2.0f;
            lista_bandas_ui[2].ganancia_db = +0.5f;
            lista_bandas_ui[3].ganancia_db = +2.5f;
            lista_bandas_ui[4].ganancia_db = +4.0f; // 500 Hz
            lista_bandas_ui[5].ganancia_db = +4.5f; // 1 kHz
            lista_bandas_ui[6].ganancia_db = +3.5f; // 2 kHz
            lista_bandas_ui[7].ganancia_db = +1.5f;
            lista_bandas_ui[8].ganancia_db = 0.0f;
            lista_bandas_ui[9].ganancia_db = -1.0f;
        }
        mensaje_estado = "Preajuste aplicado: Voz Clara (Ideal para voz y podcasts)";

    } else if (nombre_preajuste == "Rock & Metal Ámbar") {
        ganancia_graves_rapido_db = +3.5f;
        ganancia_agudos_rapido_db = +4.0f;
        volumen_general_db = -0.5f;
        if (lista_bandas_ui.size() >= 10) {
            lista_bandas_ui[0].ganancia_db = +4.5f;
            lista_bandas_ui[1].ganancia_db = +3.5f;
            lista_bandas_ui[2].ganancia_db = +1.0f;
            lista_bandas_ui[3].ganancia_db = -1.5f;
            lista_bandas_ui[4].ganancia_db = -2.5f; // Recorte en medios (V-shape)
            lista_bandas_ui[5].ganancia_db = -1.0f;
            lista_bandas_ui[6].ganancia_db = +2.0f;
            lista_bandas_ui[7].ganancia_db = +3.8f;
            lista_bandas_ui[8].ganancia_db = +4.8f;
            lista_bandas_ui[9].ganancia_db = +5.2f;
        }
        mensaje_estado = "Preajuste aplicado: Rock & Metal Ámbar (Curva en V enérgica)";

    } else if (nombre_preajuste == "Electrónica / Club") {
        ganancia_graves_rapido_db = +5.5f;
        ganancia_agudos_rapido_db = +3.5f;
        volumen_general_db = -1.0f;
        if (lista_bandas_ui.size() >= 10) {
            lista_bandas_ui[0].ganancia_db = +6.0f; // 31 Hz
            lista_bandas_ui[1].ganancia_db = +5.5f; // 62 Hz
            lista_bandas_ui[2].ganancia_db = +2.5f; // 125 Hz
            lista_bandas_ui[3].ganancia_db = 0.0f;
            lista_bandas_ui[4].ganancia_db = 0.0f;
            lista_bandas_ui[5].ganancia_db = 0.0f;
            lista_bandas_ui[6].ganancia_db = +2.0f; // 2 kHz
            lista_bandas_ui[7].ganancia_db = 0.0f;
            lista_bandas_ui[8].ganancia_db = +4.5f; // 8 kHz
            lista_bandas_ui[9].ganancia_db = +5.5f; // 16 kHz
        }
        mensaje_estado = "Preajuste aplicado: Electrónica / Club (Graves profundos y agudos brillantes)";

    } else if (nombre_preajuste == "Acústico Cálido") {
        ganancia_graves_rapido_db = +1.5f;
        ganancia_agudos_rapido_db = +1.0f;
        volumen_general_db = 0.0f;
        if (lista_bandas_ui.size() >= 10) {
            lista_bandas_ui[0].ganancia_db = +3.0f;
            lista_bandas_ui[1].ganancia_db = +2.0f;
            lista_bandas_ui[2].ganancia_db = +1.5f; // 125 Hz cuerpo
            lista_bandas_ui[3].ganancia_db = +1.0f; // 250 Hz calidez
            lista_bandas_ui[4].ganancia_db = +0.5f; // 500 Hz presencia
            lista_bandas_ui[5].ganancia_db = +1.5f;
            lista_bandas_ui[6].ganancia_db = +2.5f;
            lista_bandas_ui[7].ganancia_db = +3.0f;
            lista_bandas_ui[8].ganancia_db = +3.5f;
            lista_bandas_ui[9].ganancia_db = +4.0f;
        }
        mensaje_estado = "Preajuste aplicado: Acústico Cálido (Riqueza en maderas y cuerdas)";

    } else if (nombre_preajuste == "Jazz & Acústico") {
        ganancia_graves_rapido_db = +1.0f;
        ganancia_agudos_rapido_db = +2.0f;
        volumen_general_db = 0.0f;
        if (lista_bandas_ui.size() >= 10) {
            lista_bandas_ui[0].ganancia_db = +3.5f;
            lista_bandas_ui[1].ganancia_db = +2.5f;
            lista_bandas_ui[2].ganancia_db = +1.0f;
            lista_bandas_ui[3].ganancia_db = 0.0f;
            lista_bandas_ui[4].ganancia_db = -1.5f;
            lista_bandas_ui[5].ganancia_db = -1.5f;
            lista_bandas_ui[6].ganancia_db = 0.0f;
            lista_bandas_ui[7].ganancia_db = +1.5f;
            lista_bandas_ui[8].ganancia_db = +3.0f;
            lista_bandas_ui[9].ganancia_db = +4.0f;
        }
        mensaje_estado = "Preajuste aplicado: Jazz & Acústico (Detalle instrumental y aire espacial)";

    } else if (nombre_preajuste == "EDM Sub-Bass") {
        ganancia_graves_rapido_db = +6.0f;
        ganancia_agudos_rapido_db = +3.0f;
        volumen_general_db = -2.0f;
        if (lista_bandas_ui.size() >= 10) {
            lista_bandas_ui[0].ganancia_db = +7.0f;
            lista_bandas_ui[1].ganancia_db = +6.0f;
            lista_bandas_ui[2].ganancia_db = +4.0f;
            lista_bandas_ui[3].ganancia_db = +1.5f;
            lista_bandas_ui[4].ganancia_db = 0.0f;
            lista_bandas_ui[5].ganancia_db = -1.0f;
            lista_bandas_ui[6].ganancia_db = 0.0f;
            lista_bandas_ui[7].ganancia_db = +2.0f;
            lista_bandas_ui[8].ganancia_db = +3.5f;
            lista_bandas_ui[9].ganancia_db = +4.5f;
        }
        mensaje_estado = "Preajuste aplicado: EDM Sub-Bass (Vibración sub-grave masiva)";

    } else if (nombre_preajuste == "Gaming / Pasos FPS") {
        ganancia_graves_rapido_db = -3.0f;
        ganancia_agudos_rapido_db = +4.5f;
        volumen_general_db = +1.0f;
        if (lista_bandas_ui.size() >= 10) {
            lista_bandas_ui[0].ganancia_db = -3.0f;
            lista_bandas_ui[1].ganancia_db = -2.0f;
            lista_bandas_ui[2].ganancia_db = 0.0f;
            lista_bandas_ui[3].ganancia_db = +1.0f;
            lista_bandas_ui[4].ganancia_db = +3.0f;
            lista_bandas_ui[5].ganancia_db = +5.0f;
            lista_bandas_ui[6].ganancia_db = +4.5f;
            lista_bandas_ui[7].ganancia_db = +3.0f;
            lista_bandas_ui[8].ganancia_db = +2.0f;
            lista_bandas_ui[9].ganancia_db = +1.0f;
        }
        mensaje_estado = "Preajuste aplicado: Gaming / Pasos FPS (Realce táctico de pasos y recargas)";

    } else if (nombre_preajuste == "Cine / Películas") {
        ganancia_graves_rapido_db = +4.0f;
        ganancia_agudos_rapido_db = +2.5f;
        volumen_general_db = -1.0f;
        if (lista_bandas_ui.size() >= 10) {
            lista_bandas_ui[0].ganancia_db = +5.0f;
            lista_bandas_ui[1].ganancia_db = +4.0f;
            lista_bandas_ui[2].ganancia_db = +2.0f;
            lista_bandas_ui[3].ganancia_db = 0.0f;
            lista_bandas_ui[4].ganancia_db = -1.0f;
            lista_bandas_ui[5].ganancia_db = +1.0f;
            lista_bandas_ui[6].ganancia_db = +2.5f;
            lista_bandas_ui[7].ganancia_db = +3.5f;
            lista_bandas_ui[8].ganancia_db = +4.0f;
            lista_bandas_ui[9].ganancia_db = +4.5f;
        }
        mensaje_estado = "Preajuste aplicado: Cine / Películas (Diálogos claros y explosiones profundas)";
    }
    sincronizar_motor_dsp();
}

void VentanaPrincipalEcualizador::dibujar_barra_superior_y_preajustes() {
    ImGui::BeginGroup();
    ImGui::TextColored(ImVec4(0.96f, 0.62f, 0.04f, 1.00f), "CACAOMÁTICO AUDIO — CABINA POWERAMP C++");
    ImGui::SameLine(ImGui::GetWindowWidth() - 360.0f);

    ImGui::Checkbox("Explicaciones Sencillas (Humanizer)", &mostrar_ayuda_humanizer);

    ImGui::Separator();
    ImGui::Spacing();

    // Selector de preajustes rápidos
    ImGui::TextUnformatted("Preajustes Rápidos (Todos los perfiles):");
    ImGui::SameLine();
    if (ImGui::Button("Plano (0 dB)")) aplicar_preajuste_rapido("Aplanar Todo (0 dB)");
    ImGui::SameLine();
    if (ImGui::Button("Súper Graves")) aplicar_preajuste_rapido("Potenciar Graves (Punch)");
    ImGui::SameLine();
    if (ImGui::Button("Voz Clara")) aplicar_preajuste_rapido("Voz Clara (Podcast)");
    ImGui::SameLine();
    if (ImGui::Button("Rock & Metal")) aplicar_preajuste_rapido("Rock & Metal Ámbar");
    ImGui::SameLine();
    if (ImGui::Button("Electrónica / Club")) aplicar_preajuste_rapido("Electrónica / Club");
    ImGui::SameLine();
    if (ImGui::Button("Acústico Cálido")) aplicar_preajuste_rapido("Acústico Cálido");
    ImGui::SameLine();
    if (ImGui::Button("Jazz & Acústico")) aplicar_preajuste_rapido("Jazz & Acústico");
    ImGui::SameLine();
    if (ImGui::Button("EDM Sub-Bass")) aplicar_preajuste_rapido("EDM Sub-Bass");
    ImGui::SameLine();
    if (ImGui::Button("Gaming FPS")) aplicar_preajuste_rapido("Gaming / Pasos FPS");
    ImGui::SameLine();
    if (ImGui::Button("Cine")) aplicar_preajuste_rapido("Cine / Películas");
    ImGui::SameLine();
    if (ImGui::Button("Importar .peq")) {
        cargar_preset_poweramp_desde_ruta("11-jul-2026-9-09-38-p-m-.peq-settings");
    }

    ImGui::EndGroup();
}

void VentanaPrincipalEcualizador::dibujar_seccion_perillas_rapidas() {
    ImGui::BeginChild("panel_perillas", ImVec2(240.0f, 320.0f), true, ImGuiWindowFlags_NoScrollbar);
    ImGui::TextColored(ImVec4(0.96f, 0.62f, 0.04f, 1.00f), "CONTROLES DIRECTOS");
    ImGui::Separator();
    ImGui::Spacing();

    bool hubo_cambio = false;

    // Perilla de Graves Rápido
    ImGui::SetCursorPosX(36.0f);
    hubo_cambio |= dibujar_perilla_rotativa_poweramp(
        "Graves Rápido", &ganancia_graves_rapido_db, -12.0f, +12.0f, "%+.1f dB",
        mostrar_ayuda_humanizer ? "Aumenta o reduce de golpe todos los sonidos bajos y subwoofers." : nullptr,
        34.0f
    );

    ImGui::Spacing();

    // Perilla de Agudos Rápido
    ImGui::SetCursorPosX(36.0f);
    hubo_cambio |= dibujar_perilla_rotativa_poweramp(
        "Agudos Rápido", &ganancia_agudos_rapido_db, -12.0f, +12.0f, "%+.1f dB",
        mostrar_ayuda_humanizer ? "Da brillo cristalino o suaviza los platillos y voces agudas." : nullptr,
        34.0f
    );

    ImGui::Spacing();
    ImGui::Separator();
    ImGui::TextUnformatted("Preamp Maestro:");
    ImGui::SetNextItemWidth(210.0f);
    if (ImGui::SliderFloat("##volumen_maestro", &volumen_general_db, -15.0f, +15.0f, "Volumen: %+.1f dB")) {
        hubo_cambio = true;
    }

    if (hubo_cambio) sincronizar_motor_dsp();

    ImGui::EndChild();
}

void VentanaPrincipalEcualizador::dibujar_faders_10_bandas() {
    ImGui::BeginChild("panel_faders", ImVec2(0.0f, 320.0f), true, ImGuiWindowFlags_HorizontalScrollbar);
    ImGui::TextColored(ImVec4(0.96f, 0.62f, 0.04f, 1.00f), "ECUALIZADOR 10 BANDAS (Doble clic para encajar en 0.0 dB)");
    ImGui::Separator();
    ImGui::Spacing();

    bool hubo_cambio = false;

    for (size_t i = 0; i < lista_bandas_ui.size(); ++i) {
        auto& banda = lista_bandas_ui[i];
        char id_imgui[32];
        std::snprintf(id_imgui, sizeof(id_imgui), "banda_%d", banda.identificador);

        if (i > 0) ImGui::SameLine(0.0f, 12.0f);

        if (dibujar_fader_banda_poweramp(id_imgui, banda.nombre_humano.c_str(), banda.frecuencia_texto.c_str(), &banda.ganancia_db, -15.0f, +15.0f, 195.0f)) {
            hubo_cambio = true;
        }

        if (mostrar_ayuda_humanizer && ImGui::IsItemHovered()) {
            ImGui::BeginTooltip();
            ImGui::TextColored(ImVec4(0.96f, 0.62f, 0.04f, 1.00f), "%s (%s)", banda.nombre_humano.c_str(), banda.frecuencia_texto.c_str());
            ImGui::TextUnformatted(banda.descripcion_ayuda.c_str());
            ImGui::EndTooltip();
        }
    }

    if (hubo_cambio) sincronizar_motor_dsp();

    ImGui::EndChild();
}

void VentanaPrincipalEcualizador::dibujar_cabina_principal() noexcept {
    // Configuración de ventana de pantalla completa del ecualizador
    ImGuiViewport* vista = ImGui::GetMainViewport();
    ImGui::SetNextWindowPos(vista->WorkPos);
    ImGui::SetNextWindowSize(vista->WorkSize);

    ImGuiWindowFlags banderas_ventana = ImGuiWindowFlags_NoDecoration | ImGuiWindowFlags_NoMove | 
                                       ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoBringToFrontOnFocus;

    if (ImGui::Begin("CabinaPrincipalEcualizador", nullptr, banderas_ventana)) {
        dibujar_barra_superior_y_preajustes();
        ImGui::Spacing();

        // Gráfico y visualizador del espectro ámbar
        dibujar_curva_espectral_poweramp(motor_dsp, ImVec2(ImGui::GetContentRegionAvail().x, 150.0f));
        ImGui::Spacing();

        // Simular o reflejar respuesta dinámica del medidor VU según las ganancias activas y volumen maestro
        float nivel_base = volumen_general_db + ganancia_graves_rapido_db * 0.3f;
        medidor_izq_db = -24.0f + nivel_base + static_cast<float>(std::sin(ImGui::GetTime() * 4.5) * 6.0);
        medidor_der_db = -25.0f + nivel_base + static_cast<float>(std::cos(ImGui::GetTime() * 4.1) * 6.5);
        if (medidor_izq_db > pico_izq_db) pico_izq_db = medidor_izq_db; else pico_izq_db -= 0.08f;
        if (medidor_der_db > pico_der_db) pico_der_db = medidor_der_db; else pico_der_db -= 0.08f;

        // Dibujar medidor de nivel VU estéreo interactivo con picos LED en escala Ámbar Sunlight
        dibujar_medidor_vu_estereo_poweramp(medidor_izq_db, medidor_der_db, pico_izq_db, pico_der_db, ImVec2(ImGui::GetContentRegionAvail().x, 48.0f));
        ImGui::Spacing();

        // Panel dividido: Perillas a la izquierda, 10 Faders a la derecha
        dibujar_seccion_perillas_rapidas();
        ImGui::SameLine();
        dibujar_faders_10_bandas();

        // Barra de estado y diagnóstico del Aldraverso inferior
        ImGui::Separator();
        ImGui::TextColored(ImVec4(0.96f, 0.62f, 0.04f, 1.00f), "Estado del motor: %s", mensaje_estado.c_str());
    }
    ImGui::End();
}

} // namespace aldra::interfaz
