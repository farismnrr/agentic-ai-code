import subprocess
import time
import re
import os

ITERATIONS = 10
WARMUP = 2

def run_bench(name, cmd_args, env=None):
    # Warmup
    for _ in range(WARMUP):
        subprocess.run(cmd_args, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    
    total_time = 0
    total_rss = 0
    
    for _ in range(ITERATIONS):
        start = time.perf_counter()
        # time -v output goes to stderr
        # We need to run /usr/bin/time -v cmd_args
        time_cmd = ['/usr/bin/time', '-v'] + cmd_args
        res = subprocess.run(time_cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        end = time.perf_counter()
        total_time += (end - start)
        
        # parse rss
        rss_match = re.search(r'Maximum resident set size \(kbytes\):\s+(\d+)', res.stderr)
        if rss_match:
            total_rss += int(rss_match.group(1))
            
    avg_time_ms = (total_time / ITERATIONS) * 1000
    avg_rss_kb = total_rss / ITERATIONS
    avg_rss_mb = avg_rss_kb / 1024
    
    # get binary size
    binary_size = "N/A"
    if os.path.exists(cmd_args[0]):
        binary_size = f"{os.path.getsize(cmd_args[0]) / (1024*1024):.2f} MB"
    elif cmd_args[0] == "node":
        if os.path.exists(cmd_args[1]):
            binary_size = f"{os.path.getsize(cmd_args[1]) / 1024:.2f} KB (source)"
            
    return avg_time_ms, avg_rss_mb, binary_size
    
def print_res(name, res):
    print(f"{name:30} | {res[0]:8.2f} ms | {res[1]:8.2f} MB | {res[2]}")

print(f"{'Tool':30} | {'Latency':11} | {'Peak RSS':11} | {'Size'}")
print("-" * 70)

t_rust = run_bench("terminal-tool (Rust)", ["./target/release/terminal-tool", "--no-guard", "echo", "hello"])
t_js = run_bench("terminal-tool (Node)", ["node", "tests/terminal_cli.js", "--no-guard", "echo", "hello"])
print_res("terminal-tool (Rust)", t_rust)
print_res("terminal-tool (Node)", t_js)

c_rust = run_bench("curl-tool (Rust)", ["./target/release/curl-tool", "http://0.0.0.0"])
c_js = run_bench("curl-tool (Node)", ["node", "tests/curl_cli.js", "http://0.0.0.0"])
print_res("curl-tool (Rust)", c_rust)
print_res("curl-tool (Node)", c_js)

s_rust = run_bench("searxng-tool (Rust)", ["./target/release/searxng-search-tool", "--help"])
s_js = run_bench("searxng-tool (Node)", ["node", "tests/searxng_cli.js", "--help"])
print_res("searxng-tool (Rust)", s_rust)
print_res("searxng-tool (Node)", s_js)
