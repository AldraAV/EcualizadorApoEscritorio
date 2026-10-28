import subprocess, time

p = subprocess.Popen(["./build/motor_audio_backend.exe"], stdout=subprocess.PIPE, text=True)
start = time.time()
while time.time() - start < 3:
    linea = p.stdout.readline()
    if linea:
        print("Salida:", linea.strip()[:100])
p.kill()
