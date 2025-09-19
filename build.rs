use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Check if CUDA is available
    if let Ok(cuda_path) = env::var("CUDA_PATH") {
        println!("cargo:rustc-env=CUDA_PATH={}", cuda_path);
        build_cuda();
    } else if let Some(cuda_path) = find_cuda() {
        println!("cargo:rustc-env=CUDA_PATH={}", cuda_path);
        build_cuda();
    } else {
        println!("cargo:warning=CUDA not found. CUDA GPU acceleration will not be available.");
        println!("cargo:rustc-cfg=no_cuda");
        return;
    }
}

fn find_cuda() -> Option<String> {
    // Common CUDA installation paths
    let cuda_paths = [
        "/usr/local/cuda",
        "/opt/cuda",
        "/usr/cuda",
        "C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v12.0",
        "C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v11.8",
        "C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v11.7",
        "C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v11.6",
    ];

    for path in &cuda_paths {
        let cuda_path = PathBuf::from(path);
        if cuda_path.exists() {
            return Some(path.to_string());
        }
    }

    // Try to find nvcc in PATH
    if Command::new("nvcc").arg("--version").output().is_ok() {
        // nvcc is available, try to get CUDA path
        let which_cmd = if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        };
        if let Ok(output) = Command::new(which_cmd).arg("nvcc").output() {
            if let Ok(nvcc_path) = String::from_utf8(output.stdout) {
                let nvcc_path = nvcc_path.trim();
                if let Some(cuda_path) = PathBuf::from(nvcc_path).parent().and_then(|p| p.parent())
                {
                    return Some(cuda_path.to_string_lossy().to_string());
                }
            }
        }
    }

    None
}

fn build_cuda() {
    let cuda_path = env::var("CUDA_PATH").expect("CUDA_PATH should be set");
    let cuda_path = PathBuf::from(cuda_path);

    // Set up include and library paths
    let cuda_include = cuda_path.join("include");
    let cuda_lib = if cfg!(target_os = "windows") {
        cuda_path.join("lib").join("x64")
    } else {
        cuda_path.join("lib64")
    };

    println!("cargo:rustc-link-search=native={}", cuda_lib.display());
    println!("cargo:rustc-link-lib=cudart");
    println!("cargo:rustc-link-lib=cuda");

    // Compile CUDA source
    let nvcc = if cfg!(target_os = "windows") {
        cuda_path.join("bin").join("nvcc.exe")
    } else {
        cuda_path.join("bin").join("nvcc")
    };

    if !nvcc.exists() {
        panic!("nvcc not found at {}", nvcc.display());
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let cuda_source = "src/vanity_id.cu";
    let cuda_object = PathBuf::from(&out_dir).join("vanity_id.o");

    // Compile CUDA code to object file
    let mut nvcc_cmd = Command::new(&nvcc);
    nvcc_cmd
        .arg("-c")
        .arg(cuda_source)
        .arg("-o")
        .arg(&cuda_object)
        .arg("-I")
        .arg(&cuda_include)
        .arg("--compiler-options")
        .arg("-fPIC"); // Position independent code for shared libraries

    // Add architecture flags for better compatibility
    nvcc_cmd.arg("-gencode").arg("arch=compute_50,code=sm_50"); // Maxwell
    nvcc_cmd.arg("-gencode").arg("arch=compute_60,code=sm_60"); // Pascal
    nvcc_cmd.arg("-gencode").arg("arch=compute_70,code=sm_70"); // Volta
    nvcc_cmd.arg("-gencode").arg("arch=compute_75,code=sm_75"); // Turing
    nvcc_cmd.arg("-gencode").arg("arch=compute_80,code=sm_80"); // Ampere
    nvcc_cmd.arg("-gencode").arg("arch=compute_86,code=sm_86"); // Ampere
    nvcc_cmd.arg("-gencode").arg("arch=compute_89,code=sm_89"); // Ada Lovelace
    nvcc_cmd.arg("-gencode").arg("arch=compute_90,code=sm_90"); // Hopper

    println!("Running: {:?}", nvcc_cmd);

    let output = nvcc_cmd.output().expect("Failed to execute nvcc");

    if !output.status.success() {
        panic!(
            "nvcc compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Link the object file
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-arg={}", cuda_object.display());

    // Tell cargo to rerun if CUDA source changes
    println!("cargo:rerun-if-changed=src/vanity_id.cu");
    println!("cargo:rerun-if-changed=build.rs");

    // Enable CUDA feature
    println!("cargo:rustc-cfg=feature=\"cuda\"");
}
