#include "imgui.h"
#include "backends/imgui_impl_win32.h"
#include "backends/imgui_impl_dx11.h"
#include "estilo_aurora_cxx.hpp"
#include "ventana_principal.hpp"

#include <windows.h>
#include <d3d11.h>
#include <tchar.h>
#include <iostream>

// Declaración adelantada del manejador de mensajes de Dear ImGui para Win32
extern IMGUI_IMPL_API LRESULT ImGui_ImplWin32_WndProcHandler(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam);

namespace {

ID3D11Device*            dispositivo_d3d = nullptr;
ID3D11DeviceContext*     contexto_dispositivo_d3d = nullptr;
IDXGISwapChain*          cadena_intercambio = nullptr;
ID3D11RenderTargetView*  vista_objetivo_render = nullptr;

void crear_vista_objetivo_render() noexcept {
    ID3D11Texture2D* buffer_trasero = nullptr;
    cadena_intercambio->GetBuffer(0, IID_PPV_ARGS(&buffer_trasero));
    if (buffer_trasero) {
        dispositivo_d3d->CreateRenderTargetView(buffer_trasero, nullptr, &vista_objetivo_render);
        buffer_trasero->Release();
    }
}

void limpiar_vista_objetivo_render() noexcept {
    if (vista_objetivo_render) {
        vista_objetivo_render->Release();
        vista_objetivo_render = nullptr;
    }
}

bool inicializar_directx11(HWND hwnd) noexcept {
    DXGI_SWAP_CHAIN_DESC descripcion_cadena = {};
    descripcion_cadena.BufferCount = 2;
    descripcion_cadena.BufferDesc.Width = 0;
    descripcion_cadena.BufferDesc.Height = 0;
    descripcion_cadena.BufferDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    descripcion_cadena.BufferDesc.RefreshRate.Numerator = 60;
    descripcion_cadena.BufferDesc.RefreshRate.Denominator = 1;
    descripcion_cadena.Flags = DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH;
    descripcion_cadena.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    descripcion_cadena.OutputWindow = hwnd;
    descripcion_cadena.SampleDesc.Count = 1;
    descripcion_cadena.SampleDesc.Quality = 0;
    descripcion_cadena.Windowed = TRUE;
    descripcion_cadena.SwapEffect = DXGI_SWAP_EFFECT_DISCARD;

    UINT banderas_creacion = 0;
    D3D_FEATURE_LEVEL niveles_caracteristicas[] = { D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_0 };
    D3D_FEATURE_LEVEL nivel_obtenido;

    HRESULT resultado = D3D11CreateDeviceAndSwapChain(
        nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr, banderas_creacion,
        niveles_caracteristicas, 2, D3D11_SDK_VERSION, &descripcion_cadena,
        &cadena_intercambio, &dispositivo_d3d, &nivel_obtenido, &contexto_dispositivo_d3d
    );

    if (resultado != S_OK) return false;

    crear_vista_objetivo_render();
    return true;
}

void limpiar_directx11() noexcept {
    limpiar_vista_objetivo_render();
    if (cadena_intercambio) { cadena_intercambio->Release(); cadena_intercambio = nullptr; }
    if (contexto_dispositivo_d3d) { contexto_dispositivo_d3d->Release(); contexto_dispositivo_d3d = nullptr; }
    if (dispositivo_d3d) { dispositivo_d3d->Release(); dispositivo_d3d = nullptr; }
}

LRESULT WINAPI manejar_mensajes_ventana(HWND hwnd, UINT mensaje, WPARAM parametro_w, LPARAM parametro_l) {
    if (ImGui_ImplWin32_WndProcHandler(hwnd, mensaje, parametro_w, parametro_l)) return true;

    switch (mensaje) {
    case WM_SIZE:
        if (dispositivo_d3d != nullptr && parametro_w != SIZE_MINIMIZED) {
            limpiar_vista_objetivo_render();
            cadena_intercambio->ResizeBuffers(0, (UINT)LOWORD(parametro_l), (UINT)HIWORD(parametro_l), DXGI_FORMAT_UNKNOWN, 0);
            crear_vista_objetivo_render();
        }
        return 0;
    case WM_SYSCOMMAND:
        if ((parametro_w & 0xfff0) == SC_KEYMENU) return 0; // Deshabilitar menú de sistema al presionar ALT
        break;
    case WM_DESTROY:
        ::PostQuitMessage(0);
        return 0;
    }
    return ::DefWindowProcW(hwnd, mensaje, parametro_w, parametro_l);
}

} // namespace anónimo

int WINAPI WinMain(HINSTANCE instancia_actual, HINSTANCE, LPSTR, int comando_mostrar) {
    // 1. Registrar clase de ventana en español
    WNDCLASSEXW clase_ventana = {
        sizeof(WNDCLASSEXW), CS_CLASSDC, manejar_mensajes_ventana, 0L, 0L,
        instancia_actual, nullptr, nullptr, nullptr, nullptr,
        L"AldraAV_CabinaPowerampCXX", nullptr
    };
    ::RegisterClassExW(&clase_ventana);

    // 2. Crear ventana de la aplicación de escritorio
    HWND ventana_app = ::CreateWindowExW(
        0, L"AldraAV_CabinaPowerampCXX", L"AldraAV — Ecualizador APO Poweramp (Edición C++ Nativa)",
        WS_OVERLAPPEDWINDOW, 100, 100, 1080, 680,
        nullptr, nullptr, clase_ventana.hInstance, nullptr
    );

    // 3. Inicializar aceleración por hardware DirectX 11
    if (!inicializar_directx11(ventana_app)) {
        limpiar_directx11();
        ::UnregisterClassW(clase_ventana.lpszClassName, clase_ventana.hInstance);
        return 1;
    }

    ::ShowWindow(ventana_app, comando_mostrar);
    ::UpdateWindow(ventana_app);

    // 4. Configurar contexto de Dear ImGui
    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO();
    io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard; // Soporte para navegación con teclado
    io.IniFilename = nullptr; // No generar archivo imgui.ini temporal por defecto para evitar basura

    // 5. Aplicar tema Aurora Ámbar Sunlight del Aldraverso
    aldra::interfaz::aplicar_tema_aurora_ambar();

    ImGui_ImplWin32_Init(ventana_app);
    ImGui_ImplDX11_Init(dispositivo_d3d, contexto_dispositivo_d3d);

    // 6. Instanciar la cabina principal del ecualizador
    aldra::interfaz::VentanaPrincipalEcualizador cabina_ecualizador(48000.0);

    // 7. Bucle principal de renderizado (60 FPS fluidos)
    bool continuar_bucle = true;
    while (continuar_bucle) {
        MSG mensaje;
        while (::PeekMessage(&mensaje, nullptr, 0U, 0U, PM_REMOVE)) {
            ::TranslateMessage(&mensaje);
            ::DispatchMessage(&mensaje);
            if (mensaje.message == WM_QUIT) continuar_bucle = false;
        }
        if (!continuar_bucle) break;

        ImGui_ImplDX11_NewFrame();
        ImGui_ImplWin32_NewFrame();
        ImGui::NewFrame();

        // Renderizar la interfaz nativa Poweramp
        cabina_ecualizador.dibujar_cabina_principal();

        ImGui::Render();
        const float color_limpieza[4] = { 0.06f, 0.05f, 0.04f, 1.00f };
        contexto_dispositivo_d3d->OMSetRenderTargets(1, &vista_objetivo_render, nullptr);
        contexto_dispositivo_d3d->ClearRenderTargetView(vista_objetivo_render, color_limpieza);
        ImGui_ImplDX11_RenderDrawData(ImGui::GetDrawData());

        cadena_intercambio->Present(1, 0); // V-Sync activado para máxima suavidad visual
    }

    // 8. Limpieza ordenada final
    ImGui_ImplDX11_Shutdown();
    ImGui_ImplWin32_Shutdown();
    ImGui::DestroyContext();

    limpiar_directx11();
    ::DestroyWindow(ventana_app);
    ::UnregisterClassW(clase_ventana.lpszClassName, clase_ventana.hInstance);

    return 0;
}
