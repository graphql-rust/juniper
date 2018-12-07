param (
   [Parameter(Mandatory=$true)][string]$version,
   [Parameter(Mandatory=$true)][string]$target
)

# Location to put cargo-make binary.
$cargobindir = "$env:userprofile\.cargo\bin"

# Location to stage downloaded zip file.
$zipfile = "$env:temp\cargo-make.zip"

$url = "https://github.com/sagiegurari/cargo-make/releases/download/${version}/cargo-make-v${version}-${target}.zip"

# Download the zip file.
Invoke-WebRequest -Uri $url -OutFile $zipfile

# Extract the binary to the correct location.
Expand-Archive -Path $zipfile -DestinationPath $cargobindir

# Tell azure pipelines the PATH has changed for future steps.
Write-Host "##vso[task.setvariable variable=PATH;]%PATH%;$cargobindir"
