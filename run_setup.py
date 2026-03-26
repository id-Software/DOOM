"""
run_setup.py - Builds web-doom step by step, writing logs for Copilot to read.
Run via: c:/python314/python.exe run_setup.py
"""
import subprocess, os, json, sys

ROOT = r"c:\projects\DOOM"
WEB  = os.path.join(ROOT, "web-doom")
DOOM = os.path.join(ROOT, "linuxdoom-1.10")
LOG  = os.path.join(ROOT, "setup_log.json")

results = {}

def run(label, cmd, cwd):
    print(f"\n=== {label} ===")
    sys.stdout.flush()
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=600)
    results[label] = {
        "returncode": r.returncode,
        "stdout": r.stdout[-4000:],
        "stderr": r.stderr[-2000:],
    }
    print(f"exit: {r.returncode}")
    print(r.stdout[-2000:])
    if r.stderr:
        print("STDERR:", r.stderr[-500:])
    sys.stdout.flush()
    return r.returncode == 0

# Step 1: npm install
ok = run("npm-install", ["npm", "install"], WEB)

# Step 2: emcc build (runs build_em.ps1)
if ok:
    ok = run("emcc-build",
             ["powershell", "-ExecutionPolicy", "Bypass", "-File", "build_em.ps1"],
             DOOM)

# Save results
with open(LOG, "w") as f:
    json.dump(results, f, indent=2)

print(f"\n\nLog written to: {LOG}")
print("Success:", ok)
sys.exit(0 if ok else 1)
