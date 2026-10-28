#include "ControladorAudio.h"
#include <QDebug>

ControladorAudio::ControladorAudio(QObject *parent) : QObject(parent), reproduciendo(false) {
    motor = new MotorAudio();
    if (motor->inicializar_dispositivo()) {
        qDebug() << "Motor de audio inicializado correctamente para Qt.";
    } else {
        qDebug() << "Fallo al inicializar el motor de audio.";
    }

    // Inicializar la lista de magnitudes (por ejemplo, 64 bandas para la UI)
    for (int i = 0; i < 64; ++i) {
        lista_magnitudes.append(0.0);
    }

    // Temporizador a ~60 Hz (16 ms) para actualizar la UI
    temporizador = new QTimer(this);
    connect(temporizador, &QTimer::timeout, this, &ControladorAudio::actualizar_espectro);
}

ControladorAudio::~ControladorAudio() {
    motor->detener();
    delete motor;
}

void ControladorAudio::alternar_reproduccion() {
    if (reproduciendo) {
        motor->detener();
        temporizador->stop();
    } else {
        motor->iniciar();
        temporizador->start(16);
    }
    reproduciendo = !reproduciendo;
}

bool ControladorAudio::esta_reproduciendo() const {
    return reproduciendo;
}

QVariantList ControladorAudio::leer_magnitudes() const {
    return lista_magnitudes;
}

void ControladorAudio::actualizar_espectro() {
    DatosEspectro datos = motor->obtener_datos_espectro();
    if (datos.actualizados) {
        lista_magnitudes.clear();
        
        // Mapear el tamaño completo de la FFT (ej. 512 bins útiles) a las 64 barras de la UI
        int paso = datos.magnitudes.size() / 64;
        for (int i = 0; i < 64; ++i) {
            float suma = 0.0f;
            for (int j = 0; j < paso; ++j) {
                if (i * paso + j < datos.magnitudes.size()) {
                    suma += datos.magnitudes[i * paso + j];
                }
            }
            float promedio = suma / paso;
            lista_magnitudes.append(promedio);
        }
        
        emit magnitudesCambiadas();
    }
}
