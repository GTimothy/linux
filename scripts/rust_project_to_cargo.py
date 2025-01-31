#!/usr/bin/env python3
import json
import os

def create_workspace_toml(crates, workspace_dir):
    # Create the workspace Cargo.toml with all crates as members
    members = [crate['display_name'] for crate in crates if crate.get('is_workspace_member')]
    workspace_toml = "[workspace]\nmembers = [\n" + "".join(f'    "{name}",\n' for name in members) + "]\nresolver=\"2\""

    with open(os.path.join(workspace_dir, "Cargo.toml"), "w") as f:
        f.write(workspace_toml)
    print("Created workspace Cargo.toml")

def create_crate_files(crate, workspace_dir):
    crate_name = crate["display_name"]
    crate_dir = os.path.join(workspace_dir, crate_name)
    os.makedirs(crate_dir, exist_ok=True)

    # Use the absolute path from root_module directly in the Cargo.toml
    root_module = crate.get("root_module")
    if not root_module:
        print(f"No root_module found for crate '{crate_name}', skipping.")
        return

    # Create Cargo.toml for each crate
    cargo_toml_content = f"""[package]
name = "{crate_name}"
version = "0.1.0"
edition = "{crate.get('edition', '2021')}"

[lib]
path = "{root_module}"
proc-macro = {str(crate.get("is_proc_macro", "False")).lower()}
"""


    # Add dependencies if they exist
    if "deps" in crate and crate["deps"]:
        cargo_toml_content += "\n[dependencies]\n"
        for dep in crate["deps"]:
            dep_name = dep["name"]
            # Skip 'core' and 'std' dependencies
            if dep_name in ["core", "std"]:
                print(f"Skipping dependency '{dep_name}' for crate '{crate_name}'")
                continue
            cargo_toml_content += f'{dep_name} = {{ path = "../{dep_name}" }}\n'

    # Write Cargo.toml
    with open(os.path.join(crate_dir, "Cargo.toml"), "w") as f:
        f.write(cargo_toml_content)
    print(f"Created {crate_name}/Cargo.toml with absolute root module path '{root_module}'")

    # Create build.rs if necessary
    if "cfg" in crate or "env" in crate:
        build_rs_content = "fn main() {\n"
        # Add cfg flags
        if "cfg" in crate:
            for cfg in crate["cfg"]:
                cfg = cfg.replace('"', r'\"')
                if not ('=' in cfg):
                    build_rs_content += f'    println!("cargo:rustc-check-cfg=cfg({cfg})");\n'
                build_rs_content += f'    println!("cargo:rustc-cfg={cfg}");\n'

        # Add environment variables
        if "env" in crate:
            for key, value in crate["env"].items():
                build_rs_content += f'    println!("cargo:rustc-env={key}={value}");\n'

        build_rs_content += "}\n"

        # Write build.rs
        with open(os.path.join(crate_dir, "build.rs"), "w") as f:
            f.write(build_rs_content)
        print(f"Created {crate_name}/build.rs")

def main():
    # workspace_dir = "project-root"  # Root directory for the workspace
    workspace_dir = "."  # Root directory for the workspace
    os.makedirs(workspace_dir, exist_ok=True)

    # Load the rust-project.json
    with open("rust-project.json", "r") as f:
        data = json.load(f)

    crates = data.get("crates", [])

    # Create workspace-level Cargo.toml
    create_workspace_toml(crates, workspace_dir)

    # Create individual Cargo.toml and build.rs for each crate
    for crate in crates:
        if crate.get("is_workspace_member", False):
            create_crate_files(crate, workspace_dir)

if __name__ == "__main__":
    main()

