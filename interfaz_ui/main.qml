import QtQuick
import QtQuick.Window
import QtQuick.Controls
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects

Window {
    width: 1000
    height: 750
    visible: true
    title: qsTr("Ecualizador APO - Aurora Visual")
    color: "#0a0a0a" // Fondo oscuro profundo

    // Fondo estético (Lava Neón sutil)
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#140008" } // Cereza profundo
            GradientStop { position: 1.0; color: "#000000" } // Negro
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 15
        anchors.margins: 20

        // Cabecera Glassmorphism
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 80
            color: "#1Affffff"
            border.color: "#33ffffff"
            border.width: 1
            radius: 10

            ColumnLayout {
                anchors.centerIn: parent
                spacing: 5

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: (controladorAudio && controladorAudio.tituloPista !== "") 
                        ? controladorAudio.tituloPista + " - " + controladorAudio.artistaPista
                        : "ECUALIZADOR APO ALDRAVERSO"
                    color: "#ff2a5f"
                    font.family: "Space Grotesk"
                    font.pointSize: (controladorAudio && controladorAudio.tituloPista !== "") ? 16 : 20
                    font.bold: true
                    font.letterSpacing: 2
                    
                    layer.enabled: true
                    layer.effect: DropShadow {
                        transparentBorder: true
                        color: "#ff2a5f"
                        samples: 20
                        radius: 10
                    }
                }
                
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    visible: (controladorAudio && controladorAudio.appPista !== "")
                    text: "Origen: " + (controladorAudio ? controladorAudio.appPista : "")
                    color: "#ffb347"
                    font.family: "JetBrains Mono"
                    font.pointSize: 10
                }
            }
        }

        // Marco fijo para el Espectro Rítmico (evita saltos en el layout)
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 160
            color: "#0a000000"
            border.color: "#33ff2a5f"
            border.width: 1
            radius: 10
            clip: true // Evitar desbordamiento visual

            Row {
                anchors.centerIn: parent
                spacing: 6
                height: 140

                Repeater {
                    model: 32 // Reducido a 32 bandas para mejor rendimiento
                    Item {
                        width: 12
                        height: 140

                        Rectangle {
                            width: parent.width
                            height: {
                                if (controladorAudio && controladorAudio.magnitudes) {
                                    var mag = controladorAudio.magnitudes[index];
                                    return Math.max(5, mag * 140);
                                }
                                return 5;
                            }
                            color: "#ffb347" // Ámbar Sunlight
                            radius: 4
                            anchors.bottom: parent.bottom

                            layer.enabled: true
                            layer.effect: DropShadow {
                                transparentBorder: true
                                color: "#ffb347"
                                samples: 8
                                radius: 4
                            }
                            
                            Behavior on height {
                                NumberAnimation {
                                    duration: 60
                                    easing.type: Easing.OutQuad
                                }
                            }
                        }
                    }
                }
            }
        }

        // TabBar de Navegación
        TabBar {
            id: tabBar
            Layout.fillWidth: true
            background: Rectangle { color: "transparent" }
            
            TabButton {
                text: "Ecualizador"
                font.family: "JetBrains Mono"
                font.bold: true
                contentItem: Text {
                    text: parent.text
                    color: parent.checked ? "#ff2a5f" : "#888888"
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    color: "transparent"
                    border.width: 0
                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 3
                        color: parent.parent.checked ? "#ff2a5f" : "transparent"
                    }
                }
            }
            TabButton {
                text: "Espacio"
                font.family: "JetBrains Mono"
                font.bold: true
                contentItem: Text {
                    text: parent.text
                    color: parent.checked ? "#ff2a5f" : "#888888"
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    color: "transparent"
                    border.width: 0
                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 3
                        color: parent.parent.checked ? "#ff2a5f" : "transparent"
                    }
                }
            }
            TabButton {
                text: "Compresor"
                font.family: "JetBrains Mono"
                font.bold: true
                contentItem: Text {
                    text: parent.text
                    color: parent.checked ? "#ff2a5f" : "#888888"
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    color: "transparent"
                    border.width: 0
                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 3
                        color: parent.parent.checked ? "#ff2a5f" : "transparent"
                    }
                }
            }
            TabButton {
                text: "Reverb"
                font.family: "JetBrains Mono"
                font.bold: true
                contentItem: Text {
                    text: parent.text
                    color: parent.checked ? "#ff2a5f" : "#888888"
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    color: "transparent"
                    border.width: 0
                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 3
                        color: parent.parent.checked ? "#ff2a5f" : "transparent"
                    }
                }
            }
        }

        // Vistas de los Paneles
        SwipeView {
            id: swipeView
            currentIndex: tabBar.currentIndex
            Layout.fillWidth: true
            Layout.fillHeight: true
            interactive: false // Desactivar swipe táctil para no interferir con controles deslizantes

            PanelEQ {}
            PanelEspacio {}
            PanelCompresor {}
            PanelReverb {}
        }

        Connections {
            target: tabBar
            function onCurrentIndexChanged() {
                swipeView.currentIndex = tabBar.currentIndex
            }
        }

        // Controles Inferiores
        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 20

            Button {
                text: (controladorAudio && controladorAudio.reproduciendo) ? "Detener Motor" : "Iniciar Motor"
                font.family: "Inter"
                font.pointSize: 12
                font.bold: true

                background: Rectangle {
                    implicitWidth: 150
                    implicitHeight: 45
                    radius: 8
                    color: parent.down ? "#cc224c" : "#ff2a5f"
                    border.color: "#ff5e84"
                    border.width: 1
                }
                contentItem: Text {
                    text: parent.text
                    color: "white"
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }

                onClicked: {
                    if (controladorAudio) {
                        controladorAudio.alternar_reproduccion()
                    }
                }
            }
        }
    }
}
