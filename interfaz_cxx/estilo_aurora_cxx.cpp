#include "estilo_aurora_cxx.hpp"

namespace aldra::interfaz {

void aplicar_tema_aurora_ambar() noexcept {
    ImGuiStyle& estilo = ImGui::GetStyle();
    ImVec4* colores = estilo.Colors;

    // Redondeos esqueumórficos elegantes y sólidos
    estilo.WindowRounding    = 8.0f;
    estilo.ChildRounding     = 6.0f;
    estilo.FrameRounding     = 6.0f;
    estilo.PopupRounding     = 6.0f;
    estilo.ScrollbarRounding = 6.0f;
    estilo.GrabRounding      = 5.0f;
    estilo.TabRounding       = 6.0f;

    // Dimensiones y espaciados optimizados para ergonomía de audio
    estilo.WindowPadding     = ImVec2(14.0f, 14.0f);
    estilo.FramePadding      = ImVec2(10.0f, 6.0f);
    estilo.ItemSpacing       = ImVec2(10.0f, 8.0f);
    estilo.ItemInnerSpacing  = ImVec2(8.0f, 6.0f);
    estilo.IndentSpacing     = 20.0f;
    estilo.ScrollbarSize     = 14.0f;
    estilo.GrabMinSize       = 14.0f;
    estilo.WindowBorderSize  = 1.0f;
    estilo.ChildBorderSize   = 1.0f;
    estilo.PopupBorderSize   = 1.0f;
    estilo.FrameBorderSize   = 1.0f;

    // Paleta Aurora Ámbar Sunlight (Estrictamente Cero Púrpura, Cero Vidrio)
    colores[ImGuiCol_Text]                   = ImVec4(0.99f, 0.95f, 0.88f, 1.00f); // Crema claro solar (#FEF3E2)
    colores[ImGuiCol_TextDisabled]           = ImVec4(0.55f, 0.50f, 0.44f, 1.00f); // Gris tierra deshabilitado
    colores[ImGuiCol_WindowBg]               = ImVec4(0.06f, 0.05f, 0.04f, 1.00f); // Carbón profundo mate (#0F0E0B)
    colores[ImGuiCol_ChildBg]                = ImVec4(0.09f, 0.08f, 0.07f, 1.00f); // Panel secundario (#171513)
    colores[ImGuiCol_PopupBg]                = ImVec4(0.11f, 0.10f, 0.08f, 1.00f); // Fondo de menú emergente
    colores[ImGuiCol_Border]                 = ImVec4(0.24f, 0.21f, 0.17f, 1.00f); // Borde metálico bronce (#3D372E)
    colores[ImGuiCol_BorderShadow]           = ImVec4(0.00f, 0.00f, 0.00f, 0.40f); // Sombra sutil de profundidad
    colores[ImGuiCol_FrameBg]                = ImVec4(0.13f, 0.12f, 0.10f, 1.00f); // Fondo de controles e inputs (#211E1A)
    colores[ImGuiCol_FrameBgHovered]         = ImVec4(0.18f, 0.16f, 0.14f, 1.00f); // Hover sobre controles
    colores[ImGuiCol_FrameBgActive]          = ImVec4(0.23f, 0.20f, 0.17f, 1.00f); // Control presionado
    colores[ImGuiCol_TitleBg]                = ImVec4(0.08f, 0.07f, 0.06f, 1.00f); // Barra de título
    colores[ImGuiCol_TitleBgActive]          = ImVec4(0.14f, 0.11f, 0.07f, 1.00f); // Barra de título activa
    colores[ImGuiCol_TitleBgCollapsed]       = ImVec4(0.06f, 0.05f, 0.04f, 1.00f);
    colores[ImGuiCol_MenuBarBg]              = ImVec4(0.09f, 0.08f, 0.07f, 1.00f);
    colores[ImGuiCol_ScrollbarBg]            = ImVec4(0.05f, 0.04f, 0.03f, 1.00f);
    colores[ImGuiCol_ScrollbarGrab]          = ImVec4(0.24f, 0.21f, 0.17f, 1.00f);
    colores[ImGuiCol_ScrollbarGrabHovered]   = ImVec4(0.35f, 0.30f, 0.24f, 1.00f);
    colores[ImGuiCol_ScrollbarGrabActive]    = ImVec4(0.48f, 0.40f, 0.31f, 1.00f);
    colores[ImGuiCol_CheckMark]              = ImVec4(0.96f, 0.62f, 0.04f, 1.00f); // Ámbar resplandeciente (#F59E0B)
    colores[ImGuiCol_SliderGrab]             = ImVec4(0.96f, 0.62f, 0.04f, 1.00f); // Fader deslizador (#F59E0B)
    colores[ImGuiCol_SliderGrabActive]       = ImVec4(1.00f, 0.70f, 0.00f, 1.00f); // Fader activo brillante (#FFB300)
    colores[ImGuiCol_Button]                 = ImVec4(0.16f, 0.14f, 0.12f, 1.00f); // Botón estándar (#29241F)
    colores[ImGuiCol_ButtonHovered]          = ImVec4(0.85f, 0.47f, 0.02f, 1.00f); // Hover ámbar cálido (#D97706)
    colores[ImGuiCol_ButtonActive]           = ImVec4(0.71f, 0.33f, 0.04f, 1.00f); // Click ámbar oscuro (#B45309)
    colores[ImGuiCol_Header]                 = ImVec4(0.18f, 0.15f, 0.12f, 1.00f); // Cabecera de sección
    colores[ImGuiCol_HeaderHovered]          = ImVec4(0.85f, 0.47f, 0.02f, 0.80f);
    colores[ImGuiCol_HeaderActive]           = ImVec4(0.96f, 0.62f, 0.04f, 1.00f);
    colores[ImGuiCol_Separator]              = ImVec4(0.24f, 0.21f, 0.17f, 1.00f); // Separador metálico
    colores[ImGuiCol_SeparatorHovered]       = ImVec4(0.85f, 0.47f, 0.02f, 1.00f);
    colores[ImGuiCol_SeparatorActive]        = ImVec4(0.96f, 0.62f, 0.04f, 1.00f);
    colores[ImGuiCol_ResizeGrip]             = ImVec4(0.24f, 0.21f, 0.17f, 1.00f);
    colores[ImGuiCol_ResizeGripHovered]      = ImVec4(0.85f, 0.47f, 0.02f, 1.00f);
    colores[ImGuiCol_ResizeGripActive]       = ImVec4(0.96f, 0.62f, 0.04f, 1.00f);
    colores[ImGuiCol_Tab]                    = ImVec4(0.12f, 0.11f, 0.09f, 1.00f);
    colores[ImGuiCol_TabHovered]             = ImVec4(0.85f, 0.47f, 0.02f, 1.00f);
    colores[ImGuiCol_TabActive]              = ImVec4(0.20f, 0.17f, 0.14f, 1.00f);
    colores[ImGuiCol_TabUnfocused]           = ImVec4(0.09f, 0.08f, 0.07f, 1.00f);
    colores[ImGuiCol_TabUnfocusedActive]     = ImVec4(0.14f, 0.12f, 0.10f, 1.00f);
    colores[ImGuiCol_PlotLines]              = ImVec4(0.96f, 0.62f, 0.04f, 1.00f); // Línea de espectro ámbar
    colores[ImGuiCol_PlotLinesHovered]       = ImVec4(1.00f, 0.70f, 0.00f, 1.00f);
    colores[ImGuiCol_PlotHistogram]          = ImVec4(0.85f, 0.47f, 0.02f, 1.00f);
    colores[ImGuiCol_PlotHistogramHovered]   = ImVec4(0.96f, 0.62f, 0.04f, 1.00f);
    colores[ImGuiCol_TableHeaderBg]          = ImVec4(0.14f, 0.12f, 0.10f, 1.00f);
    colores[ImGuiCol_TableBorderStrong]      = ImVec4(0.24f, 0.21f, 0.17f, 1.00f);
    colores[ImGuiCol_TableBorderLight]       = ImVec4(0.18f, 0.16f, 0.13f, 1.00f);
    colores[ImGuiCol_TableRowBg]             = ImVec4(0.00f, 0.00f, 0.00f, 0.00f);
    colores[ImGuiCol_TableRowBgAlt]          = ImVec4(1.00f, 1.00f, 1.00f, 0.03f);
    colores[ImGuiCol_TextSelectedBg]         = ImVec4(0.85f, 0.47f, 0.02f, 0.35f);
    colores[ImGuiCol_DragDropTarget]         = ImVec4(0.96f, 0.62f, 0.04f, 1.00f);
    colores[ImGuiCol_NavHighlight]           = ImVec4(0.96f, 0.62f, 0.04f, 1.00f);
    colores[ImGuiCol_NavWindowingHighlight]  = ImVec4(1.00f, 1.00f, 1.00f, 0.70f);
    colores[ImGuiCol_NavWindowingDimBg]      = ImVec4(0.80f, 0.80f, 0.80f, 0.20f);
    colores[ImGuiCol_ModalWindowDimBg]       = ImVec4(0.06f, 0.05f, 0.04f, 0.75f);
}

} // namespace aldra::interfaz
