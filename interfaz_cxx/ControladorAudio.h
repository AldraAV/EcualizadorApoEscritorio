#pragma once

#include <QObject>
#include <QVariantList>
#include <QTimer>
#include "motor_audio.h"

class ControladorAudio : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantList magnitudes READ leer_magnitudes NOTIFY magnitudesCambiadas)

public:
    explicit ControladorAudio(QObject *parent = nullptr);
    ~ControladorAudio();

    // Invocable desde QML para iniciar/detener
    Q_INVOKABLE void alternar_reproduccion();
    Q_INVOKABLE bool esta_reproduciendo() const;

    QVariantList leer_magnitudes() const;

signals:
    void magnitudesCambiadas();

private slots:
    void actualizar_espectro();

private:
    MotorAudio* motor;
    QTimer* temporizador;
    QVariantList lista_magnitudes;
    bool reproduciendo;
};
