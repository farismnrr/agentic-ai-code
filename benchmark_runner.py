import subprocess
import time
import os
import sys
import resource

if len(sys.argv) > 1:
    cmd_args = sys.argv[1:]
    start = time.perf_counter()
    subprocess.run(cmd_args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    end = time.perf_counter()
    
    ru = resource.getrusage(resource.RUSAGE_CHILDREN)
    maxrss_kb = ru.ru_maxrss
    
    # get binary size
    binary_size = "N/A"
    if os.path.exists(cmd_args[0]):
        binary_size = f"{os.path.getsize(cmd_args[0]) / (1024*1024):.2f} MB"
    elif cmd_args[0] == "node":
        if os.path.exists(cmd_args[1]):
            binary_size = f"{os.path.getsize(cmd_args[1]) / 1024:.2f} KB (source)"
            
    print(f"{(end - start)*1000:.2f},{maxrss_kb},{binary_size}")
    sys.exit(0)

ITERATIONS = 10
WARMUP = 2

def run_bench(name, cmd_args):
    # Warmup
    for _ in range(WARMUP):
        subprocess.run(cmd_args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    
    total_time = 0
    total_rss = 0
    bin_size = "N/A"
    
    for _ in range(ITERATIONS):
        # run this script as a subprocess to isolate RUSAGE_CHILDREN
        res = subprocess.run([sys.executable, __file__] + cmd_args, stdout=subprocess.PIPE, text=True)
        out = res.stdout.strip().split(',')
        if len(out) == 3:
            total_time += float(out[0])
            total_rss += int(out[1])
            bin_size = out[2]
            
    avg_time_ms = (total_time / ITERATIONS)
    avg_rss_kb = total_rss / ITERATIONS
    avg_rss_mb = avg_rss_kb / 1024
    
    return avg_time_ms, avg_rss_mb, bin_size
    
def print_res(name, res):
    print(f"{name:30} | {res[0]:8.2f} ms | {res[1]:8.2f} MB | {res[2]}")

print(f"{'Tool':30} | {'Latency':11} | {'Peak RSS':11} | {'Size'}")
print("-" * 70)

t_rust = run_bench("terminal-tool (Rust)", ["./target/release/terminal-tool", "--no-guard", "echo", "hello"])
t_js = run_bench("terminal-tool (Node)", ["node", "packages/rust-tools/tests/terminal_cli.js", "--no-guard", "echo", "hello"])
print_res("terminal-tool (Rust)", t_rust)
print_res("terminal-tool (Node)", t_js)

c_rust = run_bench("curl-tool (Rust)", ["./target/release/curl-tool", "http://0.0.0.0"])
c_js = run_bench("curl-tool (Node)", ["node", "packages/rust-tools/tests/curl_cli.js", "http://0.0.0.0"])
print_res("curl-tool (Rust)", c_rust)
print_res("curl-tool (Node)", c_js)

s_rust = run_bench("searxng-tool (Rust)", ["./target/release/searxng-search-tool", "--help"])
s_js = run_bench("searxng-tool (Node)", ["node", "packages/rust-tools/tests/searxng_cli.js", "--help"])
print_res("searxng-tool (Rust)", s_rust)
print_res("searxng-tool (Node)", s_js)
