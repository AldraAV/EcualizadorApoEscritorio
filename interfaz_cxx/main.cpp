#include "../nucleo_dsp/motor_audio.h"
#include <iostream>
#include <thread>
#include <chrono>
#include <string>
#include <sstream>
#include <fstream>
#include <vector>

struct BandaPEQ {
    double fc;
    double gain;
    double q;
};

std::vector<BandaPEQ> eq_bandas = {
    {31.5, 0.0, 1.41},
    {62.0, 0.0, 1.41},
    {125.0, 0.0, 1.41},
    {250.0, 0.0, 1.41},
    {500.0, 0.0, 1.41},
    {1000.0, 0.0, 1.41},
    {2000.0, 0.0, 1.41},
    {4000.0, 0.0, 1.41},
    {8000.0, 0.0, 1.41},
    {16000.0, 0.0, 1.41}
};

double eq_preamp = 0.0;
double eq_bass = 0.0;
double eq_treble = 0.0;

// Espacio
double space_balance = 0.0; // -1.0 a 1.0 (L a R)
double space_expand = 0.0; // 0.0 a 1.0
bool space_mono = false;

// Dinamica (Compresor)
double comp_thresh = 0.0;
double comp_ratio = 1.0;
double comp_attack = 10.0;
double comp_release = 100.0;
double comp_makeup = 0.0;

// Reverb
double rev_size = 0.0;
double rev_predelay = 0.0;
double rev_damping = 0.0;
double rev_mix = 0.0;

void actualizar_archivo_apo() {
    std::ofstream archivo("C:\\Program Files\\EqualizerAPO\\config\\apo_config.txt");
    if (archivo.is_open()) {
        archivo << "Preamp: " << eq_preamp << " dB\n";
        
        // Tonos (Shelves)
        archivo << "Filter: ON LS Fc 125 Hz Gain " << eq_bass << " dB Q 0.707\n";
        archivo << "Filter: ON HS Fc 8000 Hz Gain " << eq_treble << " dB Q 0.707\n";

        // Parametrico (10 bandas)
        for (int i = 0; i < 10; ++i) {
            archivo << "Filter: ON PK Fc " << eq_bandas[i].fc 
                    << " Hz Gain " << eq_bandas[i].gain 
                    << " dB Q " << eq_bandas[i].q << "\n";
        }

        // Espacio
        if (space_mono) {
            archivo << "Copy: L=0.5*L+0.5*R R=0.5*L+0.5*R\n";
        } else {
            // Balance normal
            double left_gain = 1.0;
            double right_gain = 1.0;
            if (space_balance < 0) { // Hacia la izquierda, atenuar derecha
                right_gain = 1.0 + space_balance; 
            } else if (space_balance > 0) { // Hacia la derecha, atenuar izquierda
                left_gain = 1.0 - space_balance;
            }
            archivo << "Copy: L=L*" << left_gain << " R=R*" << right_gain << "\n";
            
            // Stereo expand (Mid/Side processing dummy via Copy)
            if (space_expand > 0.0) {
                // Aumento de side channel
                double mid = 1.0;
                double side = 1.0 + space_expand; 
                archivo << "Copy: M=0.5*L+0.5*R S=0.5*L-0.5*R\n";
                archivo << "Copy: L=M*" << mid << "+S*" << side << " R=M*" << mid << "-S*" << side << "\n";
            }
        }

        // Compresor (VST Placeholder o comandos nativos si existen)
        // APO no tiene compresor nativo sin plugin, dejamos los parametros para VST
        archivo << "# VST Compresor (ReaComp.dll)\n";
        archivo << "# Plugin: vst2 \"C:\\Program Files\\VSTPlugins\\ReaPlugs\\reacomp-standalone.dll\" ";
        archivo << comp_thresh << " " << comp_ratio << " " << comp_attack << " " << comp_release << " " << comp_makeup << "\n";

        // Reverb (VST Placeholder)
        archivo << "# VST Reverb (ReaVerbate.dll)\n";
        archivo << "# Plugin: vst2 \"C:\\Program Files\\VSTPlugins\\ReaPlugs\\reaverbate.dll\" ";
        archivo << rev_size << " " << rev_predelay << " " << rev_damping << " " << rev_mix << "\n";

        archivo.close();
    }
}

void hilo_lectura_stdin(MotorAudio* motor) {
    std::string linea;
    while (std::getline(std::cin, linea)) {
        std::istringstream iss(linea);
        std::string comando;
        if (iss >> comando) {
            bool cambiado = false;
            
            if (comando == "PARAM_BANDA") {
                int indice;
                double fc, ganancia, q;
                if (iss >> indice >> fc >> ganancia >> q) {
                    if (indice >= 0 && indice < 10) {
                        eq_bandas[indice] = {fc, ganancia, q};
                        cambiado = true;
                    }
                }
            } else if (comando == "BANDA") { // Soporte legacy UI por si acaso
                int indice;
                double ganancia;
                if (iss >> indice >> ganancia) {
                    if (indice >= 0 && indice < 10) {
                        eq_bandas[indice].gain = ganancia;
                        cambiado = true;
                    }
                }
            } else if (comando == "PREAMP") {
                double ganancia;
                if (iss >> ganancia) {
                    eq_preamp = ganancia;
                    cambiado = true;
                }
            } else if (comando == "TONO") {
                double graves, agudos;
                if (iss >> graves >> agudos) {
                    eq_bass = graves;
                    eq_treble = agudos;
                    cambiado = true;
                }
            } else if (comando == "ESPACIO") {
                double balance, expand;
                int mono;
                if (iss >> balance >> expand >> mono) {
                    space_balance = balance;
                    space_expand = expand;
                    space_mono = (mono != 0);
                    cambiado = true;
                }
            } else if (comando == "COMPRESOR") {
                double thresh, ratio, att, rel, makeup;
                if (iss >> thresh >> ratio >> att >> rel >> makeup) {
                    comp_thresh = thresh;
                    comp_ratio = ratio;
                    comp_attack = att;
                    comp_release = rel;
                    comp_makeup = makeup;
                    cambiado = true;
                }
            } else if (comando == "REVERB") {
                double size, predelay, damp, mix;
                if (iss >> size >> predelay >> damp >> mix) {
                    rev_size = size;
                    rev_predelay = predelay;
                    rev_damping = damp;
                    rev_mix = mix;
                    cambiado = true;
                }
            }
            
            if (cambiado) {
                actualizar_archivo_apo();
            }
        }
    }
}

int main() {
    actualizar_archivo_apo();
    MotorAudio motor;
    
    if (!motor.inicializar_dispositivo()) {
        std::cerr << "{\"error\": \"Fallo al inicializar el dispositivo\"}\n";
        return 1;
    }
    
    motor.iniciar();
    
    std::thread hilo_stdin(hilo_lectura_stdin, &motor);
    hilo_stdin.detach();
    
    while (true) {
        DatosEspectro datos = motor.obtener_datos_espectro();
        std::cout << "{\"magnitudes\": [";
        for (size_t i = 0; i < datos.magnitudes.size(); ++i) {
            std::cout << datos.magnitudes[i];
            if (i < datos.magnitudes.size() - 1) std::cout << ",";
        }
        std::cout << "]}\n";
        std::cout.flush();
        
        std::this_thread::sleep_for(std::chrono::milliseconds(16));
    }
    
    return 0;
}
