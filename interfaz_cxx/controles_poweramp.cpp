#include "controles_poweramp.hpp"
#include "nucleo_dsp/ecualizador_parametrico.hpp"
#include "imgui_internal.h"
#include <cmath>
#include <cstdio>
#include <algorithm>

namespace aldra::interfaz {

bool dibujar_perilla_rotativa_poweramp(
    const char* etiqueta,
    float* valor,
    float valor_minimo,
    float valor_maximo,
    const char* formato_texto,
    const char* descripcion_humana,
    float radio_pixeles
) noexcept {
    bool valor_cambiado = false;
    ImGuiWindow* ventana = ImGui::GetCurrentWindow();
    if (ventana->SkipItems) return false;

    ImGuiContext& g = *GImGui;
    const ImGuiStyle& estilo = g.Style;
    const ImGuiID identificador = ventana->GetID(etiqueta);

    // Calcular dimensiones totales y caja de impacto (bounding box)
    const ImVec2 tamaño_total(radio_pixeles * 2.0f, radio_pixeles * 2.0f + 45.0f);
    const ImVec2 posicion_base = ventana->DC.CursorPos;
    const ImRect caja_impacto(posicion_base, ImVec2(posicion_base.x + tamaño_total.x, posicion_base.y + tamaño_total.y));

    ImGui::ItemSize(tamaño_total, estilo.FramePadding.y);
    if (!ImGui::ItemAdd(caja_impacto, identificador)) return false;

    const bool con_hover = ImGui::ItemHoverable(caja_impacto, identificador, g.CurrentItemFlags);
    const bool activo = ImGui::IsItemActive();

    // Manejo de interacción: Doble clic para reiniciar a 0.0 dB o centro
    if (con_hover && ImGui::IsMouseDoubleClicked(ImGuiMouseButton_Left)) {
        float valor_reinicio = (valor_minimo < 0.0f && valor_maximo > 0.0f) ? 0.0f : valor_minimo;
        if (*valor != valor_reinicio) {
            *valor = valor_reinicio;
            valor_cambiado = true;
        }
    }
    // Arrastre con clic izquierdo para girar
    if (con_hover && ImGui::IsMouseClicked(ImGuiMouseButton_Left)) {
        ImGui::SetActiveID(identificador, ventana);
        ImGui::FocusWindow(ventana);
    }

    if (activo) {
        if (ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
            float factor_sensibilidad = 1.0f / (radio_pixeles * 4.0f);
            if (ImGui::GetIO().KeyShift) factor_sensibilidad *= 0.1f; // Modo de ajuste fino (precisión extrema)

            float delta = -ImGui::GetIO().MouseDelta.y * (valor_maximo - valor_minimo) * factor_sensibilidad;
            if (delta != 0.0f) {
                *valor = std::clamp(*valor + delta, valor_minimo, valor_maximo);
                valor_cambiado = true;
            }
        } else {
            ImGui::ClearActiveID();
        }
    }

    // Interacción mediante rueda de desplazamiento del ratón (Mouse Wheel)
    if (con_hover && ImGui::GetIO().MouseWheel != 0.0f) {
        float paso_rueda = (valor_maximo - valor_minimo) * 0.05f * ImGui::GetIO().MouseWheel;
        *valor = std::clamp(*valor + paso_rueda, valor_minimo, valor_maximo);
        valor_cambiado = true;
    }

    // Explicación humana (humanizer) en Tooltip flotante al mantener el cursor
    if (con_hover && descripcion_humana && descripcion_humana[0] != '\0') {
        ImGui::BeginTooltip();
        ImGui::PushTextWrapPos(ImGui::GetFontSize() * 22.0f);
        ImGui::TextColored(ImVec4(0.96f, 0.62f, 0.04f, 1.00f), "%s", etiqueta);
        ImGui::TextUnformatted(descripcion_humana);
        ImGui::PopTextWrapPos();
        ImGui::EndTooltip();
    }

    // --- RENDERIZADO GRÁFICO ESQUEUMÓRFICO (Cero Púrpura, Cero Vidrio) ---
    ImDrawList* lista_dibujo = ImGui::GetWindowDrawList();
    const ImVec2 centro_circulo(posicion_base.x + radio_pixeles, posicion_base.y + radio_pixeles);
    const float radio_interno = radio_pixeles * 0.78f;

    // Fondo y borde metálico del dial
    ImU32 color_fondo = ImGui::GetColorU32(activo ? ImGuiCol_FrameBgActive : (con_hover ? ImGuiCol_FrameBgHovered : ImGuiCol_FrameBg));
    ImU32 color_borde = ImGui::GetColorU32(ImGuiCol_Border);
    lista_dibujo->AddCircleFilled(centro_circulo, radio_pixeles, color_fondo, 48);
    lista_dibujo->AddCircle(centro_circulo, radio_pixeles, color_borde, 48, 2.0f);
    lista_dibujo->AddCircleFilled(centro_circulo, radio_interno, ImGui::GetColorU32(ImGuiCol_WindowBg), 48);

    // Mapear valor al ángulo de giro (-135° hasta +135°, en radianes)
    const float proporcion = (*valor - valor_minimo) / (valor_maximo - valor_minimo);
    const float angulo_minimo = -3.0f * IM_PI / 4.0f; // -135°
    const float angulo_maximo = +3.0f * IM_PI / 4.0f; // +135°
    const float angulo_actual = angulo_minimo + proporcion * (angulo_maximo - angulo_minimo);

    // Arco de progreso o ganancia en color Ámbar Solar resplandeciente (#F59E0B)
    ImU32 color_ambar = ImGui::GetColorU32(ImVec4(0.96f, 0.62f, 0.04f, 1.00f));
    float angulo_centro = 0.0f; // Para valores con cero en el centro (-12 a +12 dB)
    if (valor_minimo < 0.0f && valor_maximo > 0.0f) {
        float prop_cero = (0.0f - valor_minimo) / (valor_maximo - valor_minimo);
        angulo_centro = angulo_minimo + prop_cero * (angulo_maximo - angulo_minimo);
    } else {
        angulo_centro = angulo_minimo;
    }

    lista_dibujo->PathArcTo(centro_circulo, (radio_pixeles + radio_interno) * 0.5f, angulo_centro, angulo_actual, 32);
    lista_dibujo->PathStroke(color_ambar, 0, (radio_pixeles - radio_interno) * 0.85f);

    // Muesca / Indicador de aguja en el dial
    ImVec2 punta_muesca(
        centro_circulo.x + std::sin(angulo_actual) * radio_interno * 0.85f,
        centro_circulo.y - std::cos(angulo_actual) * radio_interno * 0.85f
    );
    lista_dibujo->AddLine(centro_circulo, punta_muesca, color_ambar, 3.0f);
    lista_dibujo->AddCircleFilled(centro_circulo, radio_pixeles * 0.18f, color_ambar, 16);

    // Etiquetas de texto inferiores
    char texto_valor[32];
    std::snprintf(texto_valor, sizeof(texto_valor), formato_texto, *valor);

    ImVec2 tamaño_etiqueta = ImGui::CalcTextSize(etiqueta);
    ImVec2 tamaño_valor = ImGui::CalcTextSize(texto_valor);

    lista_dibujo->AddText(
        ImVec2(posicion_base.x + (tamaño_total.x - tamaño_etiqueta.x) * 0.5f, posicion_base.y + radio_pixeles * 2.0f + 4.0f),
        ImGui::GetColorU32(ImGuiCol_Text), etiqueta
    );
    lista_dibujo->AddText(
        ImVec2(posicion_base.x + (tamaño_total.x - tamaño_valor.x) * 0.5f, posicion_base.y + radio_pixeles * 2.0f + 22.0f),
        color_ambar, texto_valor
    );

    return valor_cambiado;
}

bool dibujar_fader_banda_poweramp(
    const char* identificador_banda,
    const char* nombre_humano,
    const char* frecuencia_texto,
    float* ganancia_db,
    float minimo_db,
    float maximo_db,
    float alto_pixeles
) noexcept {
    bool modificado = false;
    ImGui::PushID(identificador_banda);

    ImGui::BeginGroup();

    // 1. Nombre humano (Humanizer al estilo Poweramp)
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.85f, 0.78f, 0.68f, 1.00f));
    ImVec2 tam_nombre = ImGui::CalcTextSize(nombre_humano);
    ImGui::SetCursorPosX(ImGui::GetCursorPosX() + (58.0f - tam_nombre.x) * 0.5f);
    ImGui::TextUnformatted(nombre_humano);
    ImGui::PopStyleColor();

    // 2. Frecuencia en Hz/kHz
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.55f, 0.50f, 0.44f, 1.00f));
    ImVec2 tam_freq = ImGui::CalcTextSize(frecuencia_texto);
    ImGui::SetCursorPosX(ImGui::GetCursorPosX() + (58.0f - tam_freq.x) * 0.5f);
    ImGui::TextUnformatted(frecuencia_texto);
    ImGui::PopStyleColor();

    ImGui::Spacing();

    // 3. Fader vertical con snap automático al centro (0.0 dB) con doble clic
    ImVec2 pos_antes_slider = ImGui::GetCursorScreenPos();
    ImGui::PushStyleColor(ImGuiCol_FrameBg, ImVec4(0.13f, 0.12f, 0.10f, 1.00f));
    ImGui::PushStyleColor(ImGuiCol_SliderGrab, ImVec4(0.96f, 0.62f, 0.04f, 1.00f));
    ImGui::PushStyleColor(ImGuiCol_SliderGrabActive, ImVec4(1.00f, 0.70f, 0.00f, 1.00f));

    if (ImGui::VSliderFloat("##deslizador", ImVec2(58.0f, alto_pixeles), ganancia_db, minimo_db, maximo_db, "")) {
        modificado = true;
    }

    // Si el usuario hace doble clic sobre el fader o cerca, encajar instantáneamente en 0.0 dB
    if (ImGui::IsItemHovered() && ImGui::IsMouseDoubleClicked(ImGuiMouseButton_Left)) {
        *ganancia_db = 0.0f;
        modificado = true;
    }

    ImGui::PopStyleColor(3);

    // Dibujar línea central de 0 dB y relleno luminoso
    ImDrawList* lista = ImGui::GetWindowDrawList();
    float prop_cero = (0.0f - minimo_db) / (maximo_db - minimo_db);
    float y_cero = pos_antes_slider.y + alto_pixeles * (1.0f - prop_cero);

    // Línea horizontal indicadora de 0 dB (encaje magnético visual)
    lista->AddLine(
        ImVec2(pos_antes_slider.x + 8.0f, y_cero),
        ImVec2(pos_antes_slider.x + 50.0f, y_cero),
        ImGui::GetColorU32(ImVec4(0.40f, 0.36f, 0.30f, 1.00f)), 2.0f
    );

    ImGui::Spacing();

    // 4. Ganancia actual en dB
    char texto_db[24];
    if (std::abs(*ganancia_db) < 0.05f) {
        std::snprintf(texto_db, sizeof(texto_db), "0.0 dB");
    } else {
        std::snprintf(texto_db, sizeof(texto_db), "%+.1f dB", *ganancia_db);
    }

    ImU32 color_ganancia = (std::abs(*ganancia_db) < 0.05f) 
        ? ImGui::GetColorU32(ImVec4(0.60f, 0.55f, 0.48f, 1.00f)) 
        : ImGui::GetColorU32(ImVec4(0.96f, 0.62f, 0.04f, 1.00f));

    ImVec2 tam_db = ImGui::CalcTextSize(texto_db);
    ImGui::SetCursorPosX(ImGui::GetCursorPosX() + (58.0f - tam_db.x) * 0.5f);
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(color_ganancia), "%s", texto_db);

    ImGui::EndGroup();
    ImGui::PopID();
    return modificado;
}

void dibujar_curva_espectral_poweramp(
    const aldra::audio::EcualizadorParametrico& ecualizador,
    const ImVec2& dimensiones
) noexcept {
    ImGuiWindow* ventana = ImGui::GetCurrentWindow();
    if (ventana->SkipItems) return;

    const ImVec2 pos_base = ImGui::GetCursorScreenPos();
    ImDrawList* lista = ImGui::GetWindowDrawList();

    // Fondo del analizador espectral mate
    lista->AddRectFilled(
        pos_base,
        ImVec2(pos_base.x + dimensiones.x, pos_base.y + dimensiones.y),
        ImGui::GetColorU32(ImVec4(0.06f, 0.05f, 0.04f, 1.00f)), 6.0f
    );
    lista->AddRect(
        pos_base,
        ImVec2(pos_base.x + dimensiones.x, pos_base.y + dimensiones.y),
        ImGui::GetColorU32(ImVec4(0.24f, 0.21f, 0.17f, 1.00f)), 6.0f, 0, 1.5f
    );

    // Rejilla de frecuencias de referencia
    const double frecuencias_guia[] = { 31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0 };
    const char* etiquetas_guia[] = { "31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k" };
    const double log_min = std::log10(20.0);
    const double log_max = std::log10(20000.0);

    for (size_t i = 0; i < 10; ++i) {
        double prop_x = (std::log10(frecuencias_guia[i]) - log_min) / (log_max - log_min);
        float x_guia = pos_base.x + static_cast<float>(prop_x) * dimensiones.x;

        // Línea vertical tenue de la grilla
        lista->AddLine(
            ImVec2(x_guia, pos_base.y + 4.0f),
            ImVec2(x_guia, pos_base.y + dimensiones.y - 18.0f),
            ImGui::GetColorU32(ImVec4(1.00f, 1.00f, 1.00f, 0.06f)), 1.0f
        );

        // Texto de frecuencia en la base
        ImVec2 tam_txt = ImGui::CalcTextSize(etiquetas_guia[i]);
        lista->AddText(
            ImVec2(x_guia - tam_txt.x * 0.5f, pos_base.y + dimensiones.y - 16.0f),
            ImGui::GetColorU32(ImVec4(0.55f, 0.50f, 0.44f, 0.80f)), etiquetas_guia[i]
        );
    }

    // Línea central de 0 dB (respuesta plana)
    float y_cero = pos_base.y + dimensiones.y * 0.5f;
    lista->AddLine(
        ImVec2(pos_base.x, y_cero),
        ImVec2(pos_base.x + dimensiones.x, y_cero),
        ImGui::GetColorU32(ImVec4(0.96f, 0.62f, 0.04f, 0.35f)), 1.5f
    );
    lista->AddText(
        ImVec2(pos_base.x + 6.0f, y_cero - 16.0f),
        ImGui::GetColorU32(ImVec4(0.96f, 0.62f, 0.04f, 0.70f)), "0 dB (Plano)"
    );

    // Muestrear y dibujar la polilínea de respuesta en frecuencia
    constexpr int NUM_PUNTOS = 160;
    std::vector<ImVec2> puntos_curva;
    puntos_curva.reserve(NUM_PUNTOS);

    const float min_ganancia_grafico = -18.0f;
    const float max_ganancia_grafico = +18.0f;

    for (int i = 0; i < NUM_PUNTOS; ++i) {
        double prop_i = static_cast<double>(i) / (NUM_PUNTOS - 1);
        double freq_hz = std::pow(10.0, log_min + prop_i * (log_max - log_min));
        double respuesta_db = ecualizador.evaluar_respuesta_db(freq_hz);

        float x = pos_base.x + static_cast<float>(prop_i) * dimensiones.x;
        float prop_y = static_cast<float>((respuesta_db - min_ganancia_grafico) / (max_ganancia_grafico - min_ganancia_grafico));
        float y = pos_base.y + dimensiones.y * (1.0f - std::clamp(prop_y, 0.02f, 0.98f));

        puntos_curva.push_back(ImVec2(x, y));
    }

    // Dibujar trazo resplandeciente en ámbar solar
    lista->AddPolyline(puntos_curva.data(), NUM_PUNTOS, ImGui::GetColorU32(ImVec4(0.96f, 0.62f, 0.04f, 1.00f)), 0, 2.8f);

    // Avance del cursor de ImGui para respetar el área en el layout general
    ImGui::Dummy(dimensiones);
}

void dibujar_medidor_vu_estereo_poweramp(
    float nivel_izquierdo_db,
    float nivel_derecho_db,
    float pico_izquierdo_db,
    float pico_derecho_db,
    const ImVec2& dimensiones
) noexcept {
    ImGuiWindow* ventana = ImGui::GetCurrentWindow();
    if (ventana->SkipItems) return;

    const ImVec2 pos_base = ImGui::GetCursorScreenPos();
    ImDrawList* lista = ImGui::GetWindowDrawList();

    // Fondo oscuro del medidor (cero vidrio, carbono mate)
    lista->AddRectFilled(
        pos_base,
        ImVec2(pos_base.x + dimensiones.x, pos_base.y + dimensiones.y),
        ImGui::GetColorU32(ImVec4(0.06f, 0.05f, 0.04f, 1.00f)), 4.0f
    );
    lista->AddRect(
        pos_base,
        ImVec2(pos_base.x + dimensiones.x, pos_base.y + dimensiones.y),
        ImGui::GetColorU32(ImVec4(0.24f, 0.21f, 0.17f, 1.00f)), 4.0f, 0, 1.0f
    );

    const float alto_barra = (dimensiones.y - 12.0f) * 0.5f;
    const float ancho_util = dimensiones.x - 48.0f;
    const float x_inicio = pos_base.x + 40.0f;

    // Dibujar etiquetas L y R en bronce claro
    lista->AddText(ImVec2(pos_base.x + 8.0f, pos_base.y + 4.0f + alto_barra * 0.5f - 7.0f),
                   ImGui::GetColorU32(ImVec4(0.85f, 0.78f, 0.68f, 1.00f)), "IZQ (L)");
    lista->AddText(ImVec2(pos_base.x + 8.0f, pos_base.y + 8.0f + alto_barra + alto_barra * 0.5f - 7.0f),
                   ImGui::GetColorU32(ImVec4(0.85f, 0.78f, 0.68f, 1.00f)), "DER (R)");

    auto calcular_prop_db = [](float db) -> float {
        return std::clamp((db + 60.0f) / 60.0f, 0.0f, 1.0f);
    };

    const float prop_izq = calcular_prop_db(nivel_izquierdo_db);
    const float prop_der = calcular_prop_db(nivel_derecho_db);
    const float prop_pico_izq = calcular_prop_db(pico_izquierdo_db);
    const float prop_pico_der = calcular_prop_db(pico_derecho_db);

    // Dibujar segmentos de LED de -60 a 0 dB
    constexpr int NUM_SEGMENTOS = 32;
    for (int i = 0; i < NUM_SEGMENTOS; ++i) {
        float prop_s = static_cast<float>(i) / NUM_SEGMENTOS;
        float x_seg = x_inicio + prop_s * ancho_util;
        float ancho_seg = (ancho_util / NUM_SEGMENTOS) - 2.0f;

        // Color del segmento LED (Verde -> Ámbar -> Rojo al final)
        ImU32 color_led_activo;
        if (prop_s < 0.70f) {
            color_led_activo = ImGui::GetColorU32(ImVec4(0.18f, 0.75f, 0.35f, 0.95f)); // Verde señal limpia
        } else if (prop_s < 0.90f) {
            color_led_activo = ImGui::GetColorU32(ImVec4(0.96f, 0.62f, 0.04f, 0.95f)); // Ámbar solar advertencia
        } else {
            color_led_activo = ImGui::GetColorU32(ImVec4(0.92f, 0.20f, 0.18f, 0.95f)); // Rojo recorte / clip
        }
        ImU32 color_led_inactivo = ImGui::GetColorU32(ImVec4(0.13f, 0.12f, 0.10f, 0.40f));

        // Barra Izquierda
        bool activo_izq = prop_izq >= prop_s;
        lista->AddRectFilled(
            ImVec2(x_seg, pos_base.y + 4.0f),
            ImVec2(x_seg + ancho_seg, pos_base.y + 4.0f + alto_barra),
            activo_izq ? color_led_activo : color_led_inactivo, 1.5f
        );

        // Barra Derecha
        bool activo_der = prop_der >= prop_s;
        lista->AddRectFilled(
            ImVec2(x_seg, pos_base.y + 8.0f + alto_barra),
            ImVec2(x_seg + ancho_seg, pos_base.y + 8.0f + alto_barra * 2.0f),
            activo_der ? color_led_activo : color_led_inactivo, 1.5f
        );
    }

    // Indicadores de pico retenido (Peak Hold)
    if (prop_pico_izq > 0.02f) {
        float x_pico = x_inicio + prop_pico_izq * ancho_util;
        lista->AddLine(ImVec2(x_pico, pos_base.y + 3.0f), ImVec2(x_pico, pos_base.y + 5.0f + alto_barra),
                       ImGui::GetColorU32(ImVec4(1.00f, 0.85f, 0.30f, 1.00f)), 2.5f);
    }
    if (prop_pico_der > 0.02f) {
        float x_pico = x_inicio + prop_pico_der * ancho_util;
        lista->AddLine(ImVec2(x_pico, pos_base.y + 7.0f + alto_barra), ImVec2(x_pico, pos_base.y + 9.0f + alto_barra * 2.0f),
                       ImGui::GetColorU32(ImVec4(1.00f, 0.85f, 0.30f, 1.00f)), 2.5f);
    }

    ImGui::Dummy(dimensiones);
}

} // namespace aldra::interfaz
