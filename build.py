#!/usr/bin/env python

import subprocess
import os

def run_command(command):
    """Run a shell command and print its output."""
    process = subprocess.Popen(command, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    stdout, stderr = process.communicate()
    
    if process.returncode != 0:
        print(f"Error running command: {command}")
        print(stderr.decode())
    else:
        print(stdout.decode())

def checkDeps():
    """Check for required dependencies."""
    dependencies = ["figlet", "argc", "wget", "curl", "telnet", "ssh", "mosh", "bash"]
    missing_deps = []
    
    for dep in dependencies:
        if subprocess.call(f"which {dep}", shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE) != 0:
            missing_deps.append(dep)
    
    if missing_deps:
        print("Missing dependencies:", ", ".join(missing_deps))
        return False
    return True

def main():
    if not checkDeps():
        print("Please install the missing dependencies and try again.")
        return
    
    print("All dependencies are satisfied.")
    
    # Example build steps
    print("Building the project...")
    run_command("echo 'Compiling source code...'")
    if not os.path.exists("dist"):
        os.makedirs("dist")
    run_command("argc --argc-build Main.sh dist/vmodem99-a")
    print("Build completed successfully.")

if __name__ == "__main__":
    main()
