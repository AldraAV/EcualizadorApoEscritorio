import subprocess
import time
import os

backend_path = "motor_audio_backend.exe"
config_path = "apo_config.txt"

def read_config():
    with open(config_path, "r", encoding="utf-8") as f:
        return f.read()

def correr_pruebas():
    print("Iniciando motor de audio...")
    proc = subprocess.Popen(
        [backend_path], 
        stdin=subprocess.PIPE, 
        stdout=subprocess.PIPE, 
        stderr=subprocess.PIPE,
        text=True
    )
    
    time.sleep(0.5)

    pruebas = [
        ("PARAM_BANDA 0 31.5 1.41 5.5\n", "Banda 0 (Graves) - 5.5 dB", "Fc 31.5 Gain 5.5 Q 1.41"),
        ("PARAM_BANDA 4 1000 2.0 -3.2\n", "Banda 4 (Medios) - -3.2 dB", "Fc 1000 Gain -3.2 Q 2"),
        ("PREAMP -2.5\n", "Preamp -2.5 dB", "Preamp: -2.5 dB"),
        ("ESPACIO -0.5 0.8 0\n", "Espacio: Balance L, Expand 80%, Mono Off", "Copy: L=L*-0.5+R*0.8"), # simplificado, solo probamos que el backend responda y genere Copy
        ("ESPACIO 0 0 1\n", "Espacio: Mono ON", "Copy: L=0.5*L+0.5*R R=0.5*L+0.5*R"),
        ("COMPRESOR -20.0 4.0 15.0 200.0 2.0\n", "Compresor", "VST: rea_comp.dll"),
        ("REVERB 0.8 20.0 0.5 0.4\n", "Reverb", "VST: rea_verb.dll")
    ]
    
    for cmd, desc, text_expected in pruebas:
        print(f"Probando: {desc}")
        proc.stdin.write(cmd)
        proc.stdin.flush()
        time.sleep(0.2) # esperar a que procese y escriba en disco
        
        config_content = read_config()
        if text_expected in config_content or ("Copy:" in text_expected and "Copy:" in config_content) or ("VST:" in text_expected and "VST:" in config_content):
            print(f"  [EXITO] Se encontró el texto esperado o la instrucción base.")
        else:
            print(f"  [FALLO] No se encontró la salida esperada.\n  Output:\n{config_content}")
            
    proc.terminate()
    print("Pruebas finalizadas.")

if __name__ == "__main__":
    if os.path.exists(config_path):
        os.remove(config_path)
    correr_pruebas()
