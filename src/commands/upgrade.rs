use anyhow::Result;
use colored::*;

use crate::upgrade::{check_upgrade, get_installed_version, upgrade as do_upgrade};

/// Run the upgrade command
pub async fn run(check_only: bool) -> Result<()> {
    let installed = get_installed_version()?;

    if check_only {
        println!("Installed version: {}", installed.to_string().cyan());
        print!("Checking for updates... ");

        match check_upgrade().await {
            Ok(Some(latest)) => {
                println!("{}", "update available!".green());
                println!();
                println!(
                    "  {} → {}",
                    installed.to_string().yellow(),
                    latest.to_string().green()
                );
                println!();
                println!("Run {} to upgrade", "fresher upgrade".cyan());
            }
            Ok(None) => {
                println!("{}", "up to date".green());
            }
            Err(e) => {
                println!("{}", "failed".red());
                eprintln!("Error checking for updates: {}", e);
                return Err(e);
            }
        }
    } else {
        match do_upgrade().await {
            Ok(version) => {
                if version == installed {
                    println!("Already at the latest version ({})", version.to_string().green());
                } else {
                    println!();
                    println!(
                        "{} Successfully upgraded to version {}",
                        "✓".green(),
                        version.to_string().green()
                    );
                }
            }
            Err(e) => {
                eprintln!("{} Upgrade failed: {}", "✗".red(), e);
                return Err(e);
            }
        }
    }

    Ok(())
}
