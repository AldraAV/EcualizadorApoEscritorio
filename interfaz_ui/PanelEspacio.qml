import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    Rectangle {
        anchors.fill: parent
        color: "#0Affffff"
        border.color: "#1Affffff"
        radius: 10

        ColumnLayout {
            anchors.centerIn: parent
            spacing: 40

            Text {
                text: "ESPACIO Y TIEMPO"
                color: "#ff2a5f"
                font.family: "Space Grotesk"
                font.pointSize: 18
                font.bold: true
                Layout.alignment: Qt.AlignHCenter
            }

            RowLayout {
                spacing: 50
                Layout.alignment: Qt.AlignHCenter

                // Balance
                ColumnLayout {
                    spacing: 10
                    Layout.alignment: Qt.AlignHCenter
                    Dial {
                        id: dialBalance
                        from: -1.0
                        to: 1.0
                        value: 0.0
                        Layout.preferredWidth: 100
                        Layout.preferredHeight: 100
                        onValueChanged: actualizarEspacio()
                    }
                    Text {
                        text: "Balance\n" + (dialBalance.value < 0 ? "L " : (dialBalance.value > 0 ? "R " : "Centro ")) + Math.abs(dialBalance.value).toFixed(2)
                        color: "#ffffff"
                        font.family: "Inter"
                        font.pointSize: 10
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Stereo Expand
                ColumnLayout {
                    spacing: 10
                    Layout.alignment: Qt.AlignHCenter
                    Dial {
                        id: dialExpand
                        from: 0.0
                        to: 1.0
                        value: 0.0
                        Layout.preferredWidth: 100
                        Layout.preferredHeight: 100
                        onValueChanged: actualizarEspacio()
                    }
                    Text {
                        text: "Stereo Expand\n" + (dialExpand.value * 100).toFixed(0) + "%"
                        color: "#ffffff"
                        font.family: "Inter"
                        font.pointSize: 10
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }
                }
            }

            // Mono Toggle
            Button {
                id: btnMono
                text: checked ? "MONO: ON" : "MONO: OFF"
                checkable: true
                checked: false
                Layout.alignment: Qt.AlignHCenter
                font.family: "JetBrains Mono"
                font.pointSize: 12
                font.bold: true

                background: Rectangle {
                    implicitWidth: 180
                    implicitHeight: 45
                    radius: 8
                    color: parent.checked ? "#ff2a5f" : "#222222"
                    border.color: parent.checked ? "#ff5e84" : "#444444"
                    border.width: 1
                }
                contentItem: Text {
                    text: parent.text
                    color: "white"
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                
                onCheckedChanged: actualizarEspacio()
            }
        }
    }

    function actualizarEspacio() {
        if (controladorAudio) {
            controladorAudio.cambiar_espacio(dialBalance.value, dialExpand.value, btnMono.checked ? 1 : 0)
        }
    }
}
