from pathlib import Path
import shutil
import stat


def main() -> None:
    repository_root = Path(__file__).resolve().parent.parent
    target_directory = repository_root / "target"
    target_directory.mkdir(exist_ok=True)

    shell_installer = target_directory / "reportal-installer.sh"
    powershell_installer = target_directory / "reportal-installer.ps1"

    shutil.copy2(repository_root / "installer" / "install.sh", shell_installer)
    shutil.copy2(repository_root / "installer" / "install.ps1", powershell_installer)

    current_mode = shell_installer.stat().st_mode
    shell_installer.chmod(current_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


if __name__ == "__main__":
    main()
