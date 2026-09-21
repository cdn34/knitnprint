# Installing Terraform on WSL Ubuntu

Last updated: 2026-08-30

These notes document the recommended Terraform installation for WSL Ubuntu and the commands used to diagnose and repair the malformed HashiCorp APT source encountered on this workstation.

The workstation used for this installation has:

- WSL with Ubuntu 24.04 (`noble`).
- AMD64 architecture.
- Terraform `1.16.0`, installed as Debian package `1.16.0-1`.
- Terraform held at that version to prevent an unexpected package upgrade during the staging deployment work.

HashiCorp's official APT repository is preferable to an unofficial package or installer because APT verifies the signed repository metadata and package hashes and integrates the installation with Ubuntu's package management.

Official references:

- [Install Terraform](https://developer.hashicorp.com/terraform/tutorials/aws-get-started/install-cli)
- [Terraform releases](https://releases.hashicorp.com/terraform/)
- [HashiCorp package-signing keys](https://www.hashicorp.com/en/trust/security)

## Normal installation

### 1. Identify the Ubuntu release and architecture

Do not assume that every WSL installation uses `noble` or `amd64`.

```bash
grep -E '^(ID|VERSION_ID|VERSION_CODENAME|UBUNTU_CODENAME)=' /etc/os-release
dpkg --print-architecture
```

This workstation reported:

```text
VERSION_ID="24.04"
VERSION_CODENAME=noble
ID=ubuntu
UBUNTU_CODENAME=noble
amd64
```

### 2. Install the repository prerequisites

```bash
sudo apt-get update
sudo apt-get install -y gnupg software-properties-common wget
```

### 3. Install HashiCorp's APT signing key

```bash
wget -O- https://apt.releases.hashicorp.com/gpg \
  | gpg --dearmor \
  | sudo tee /usr/share/keyrings/hashicorp-archive-keyring.gpg >/dev/null
```

Verify the installed key:

```bash
gpg --no-default-keyring \
  --keyring /usr/share/keyrings/hashicorp-archive-keyring.gpg \
  --fingerprint
```

The package-signing fingerprint must be:

```text
798A EC65 4E5C 1542 8C8E 42EE AA16 FCBC A621 E701
```

Stop if the fingerprint differs. Do not add the repository or install a package signed by an unexpected key.

### 4. Add the HashiCorp repository

Using variables and `printf` avoids hard-coding the architecture and Ubuntu codename. It also ensures APT's entire repository entry is written as exactly one line.

```bash
source /etc/os-release
terraform_architecture="$(dpkg --print-architecture)"
terraform_ubuntu_codename="${UBUNTU_CODENAME:-$VERSION_CODENAME}"

printf 'deb [arch=%s signed-by=%s] %s %s main\n' \
  "$terraform_architecture" \
  '/usr/share/keyrings/hashicorp-archive-keyring.gpg' \
  'https://apt.releases.hashicorp.com' \
  "$terraform_ubuntu_codename" \
  | sudo tee /etc/apt/sources.list.d/hashicorp.list
```

Check that the file contains one complete line:

```bash
wc -l /etc/apt/sources.list.d/hashicorp.list
sudo sed -n '1p' /etc/apt/sources.list.d/hashicorp.list
```

Expected result on this workstation:

```text
1 /etc/apt/sources.list.d/hashicorp.list
deb [arch=amd64 signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com noble main
```

### 5. Refresh APT and inspect the available versions

```bash
sudo apt-get update
apt-cache madison terraform | head -10
```

Confirm that the intended version exists before installing it. The staging work selected package `1.16.0-1`.

### 6. Install and hold the selected version

```bash
sudo apt-get install -y terraform=1.16.0-1
sudo apt-mark hold terraform
```

The explicit package version makes the initial installation repeatable. The APT hold prevents a later general system upgrade from silently changing the Terraform CLI version. A deliberate upgrade can be performed later with:

```bash
sudo apt-mark unhold terraform
```

Do not unhold or upgrade Terraform in the middle of an infrastructure change without first reviewing the release notes and the project's `required_version` constraint.

### 7. Verify the installation

```bash
command -v terraform
terraform version
dpkg-query -W -f='${Package} ${Version} ${Architecture}\n' terraform
apt-mark showhold | grep -x terraform
apt-cache policy terraform | sed -n '1,8p'
```

The important output from this workstation was:

```text
/usr/bin/terraform
Terraform v1.16.0
on linux_amd64
terraform 1.16.0-1 amd64
terraform
```

`apt-cache policy` also showed that both the installed and candidate versions were `1.16.0-1` from `https://apt.releases.hashicorp.com noble/main`.

## Repairing a malformed HashiCorp APT source

### Symptom

`apt-get update` failed with:

```text
E: Malformed entry 1 in list file /etc/apt/sources.list.d/hashicorp.list (URI)
E: The list of sources could not be read.
```

The repository entry had accidentally been written across multiple physical lines:

```text
deb [arch=amd64 signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg]
  https://apt.releases.hashicorp.com noble
  main
```

APT source entries in `.list` files must be complete on one line.

### 1. Inspect the source and platform values

These were the first diagnostic commands used:

```bash
sudo sed -n '1,5p' /etc/apt/sources.list.d/hashicorp.list
grep -E '^(ID|VERSION_ID|VERSION_CODENAME|UBUNTU_CODENAME)=' /etc/os-release
dpkg --print-architecture
```

### 2. Inspect the file's exact line boundaries and bytes

The displayed text was then checked at the byte level to distinguish terminal wrapping from actual newline characters:

```bash
od -An -tx1c /etc/apt/sources.list.d/hashicorp.list
sed -n 'l' /etc/apt/sources.list.d/hashicorp.list
ls -l /etc/apt/sources.list.d/hashicorp.list
```

In `od` output, an actual newline appears as byte `0a` and as `\n` in the character view. The inspection confirmed a newline between the closing `]` and the repository URI.

`sed -n 'l'` may insert a backslash when it visually wraps a long line. The `$` marker identifies the real end of a line, so `wc -l` is a clearer final check.

### 3. Prepare a known-good one-line replacement

The repair used a temporary file so only one short privileged copy command needed to run in the user's terminal. This equivalent shell command creates that file without relying on one long pasted source line:

```bash
printf '%s%s%s\n' \
  'deb [arch=amd64 signed-by=' \
  '/usr/share/keyrings/hashicorp-archive-keyring.gpg] ' \
  'https://apt.releases.hashicorp.com noble main' \
  > /tmp/hashicorp.list
```

Verify the temporary file before copying it:

```bash
wc -l /tmp/hashicorp.list
sed -n '1p' /tmp/hashicorp.list
```

The expected line count is `1`.

### 4. Replace the malformed system file

```bash
sudo cp /tmp/hashicorp.list /etc/apt/sources.list.d/hashicorp.list
```

Confirm the replacement:

```bash
wc -l /etc/apt/sources.list.d/hashicorp.list
sed -n '1p' /etc/apt/sources.list.d/hashicorp.list
```

### 5. Refresh the package index

```bash
sudo apt-get update
```

The absence of the `Malformed entry` error confirms that APT can parse the repaired source.

### 6. Confirm the version directly from repository metadata

The following read-only commands were used to inspect the versions published for Ubuntu `noble` on AMD64 without relying on the local APT cache:

```bash
curl -fsSL https://apt.releases.hashicorp.com/dists/noble/main/binary-amd64/Packages.gz \
  | gzip -dc \
  | awk '$1 == "Package:" && $2 == "terraform" { found=1; next } found && $1 == "Version:" { print $2; found=0 }' \
  | sort -V \
  | tail -10
```

The last entry was:

```text
1.16.0-1
```

The package was then installed, held, and verified using the commands from steps 6 and 7 of the normal installation procedure.

### 7. Remove the temporary repair file

```bash
rm /tmp/hashicorp.list
```

Only remove this exact temporary file after confirming that `/etc/apt/sources.list.d/hashicorp.list` is correct and `apt-get update` succeeds.

## Troubleshooting rules

- If APT reports a malformed source, inspect the source file before retrying the update.
- If the Ubuntu codename is empty or unsupported, inspect `/etc/os-release`; do not substitute a random release name.
- If the signing fingerprint differs, stop instead of bypassing verification.
- If the desired package does not appear in `apt-cache madison terraform`, do not install a different version silently.
- A password prompt must be completed in the user's own terminal. Never send a password through a command, chat message, environment variable, or temporary file.
- Installing Terraform does not configure AWS credentials and does not contact or modify the AWS account.
