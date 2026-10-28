#ifndef CARGADOR_PEQ_HPP
#define CARGADOR_PEQ_HPP

#include "ecualizador_parametrico.hpp"
#include "tipos_audio.hpp"
#include <string>
#include <vector>
#include <stdexcept>

namespace aldra::audio {

/**
 * Excepción lanzada cuando un archivo o formato de preset .peq-settings de Poweramp es inválido.
 */
class ErrorFormatoPEQ : public std::runtime_error {
public:
    explicit ErrorFormatoPEQ(const std::string& mensaje) : std::runtime_error(mensaje) {}
};

/**
 * Estructura para representar una banda importada desde el preset.
 */
struct ConfiguracionBanda {
    int identificador;
    double frecuencia_hz;
    double factor_q;
    double ganancia_db;
    TipoFiltro tipo;
};

/**
 * Estructura para representar los datos decodificados y extraídos de un archivo de configuración .peq-settings.
 */
struct ConfiguracionPEQ {
    std::string nombre_perfil;
    double ganancia_preamplificador_db{0.0};
    double frecuencia_muestreo_hz{48000.0};
    std::vector<ConfiguracionBanda> bandas;
};

/**
 * Clase estática modular para decodificar, analizar e importar archivos de presets 
 * nativos de Poweramp (.peq-settings o Base64/JSON) hacia el EcualizadorParametrico.
 */
class CargadorPEQ {
public:
    /**
     * Carga un archivo .peq-settings desde la ruta proporcionada, lo decodifica y devuelve la estructura configurada.
     * @throws ErrorFormatoPEQ si el archivo no existe o no puede ser interpretado.
     */
    static ConfiguracionPEQ cargar_desde_archivo(const std::string& ruta_archivo);

    /**
     * Decodifica una cadena en formato Base64 cruda (como la que se almacena en el archivo .peq-settings).
     */
    static std::vector<unsigned char> decodificar_base64(const std::string& cadena_base64);

    /**
     * Aplica directamente una configuración PEQ cargada sobre una instancia del ecualizador paramétrico.
     */
    static void aplicar_sobre_ecualizador(EcualizadorParametrico& ecualizador, const ConfiguracionPEQ& configuracion);

private:
    /**
     * Extrae y reconstruye la lista de bandas a partir del búfer binario decodificado de Poweramp
     * o mediante heurística de coma flotante IEEE-754/JSON.
     */
    static ConfiguracionPEQ interpretar_buffer_decodificado(const std::vector<unsigned char>& buffer);
};

} // namespace aldra::audio

#endif // CARGADOR_PEQ_HPP
