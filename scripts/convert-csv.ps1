param([string]$Path = ".\")
$env:RUST_LOG = 'info'
$files = Get-ChildItem $Path
foreach ($file in $files)
{
    $fn = $Path + "\" + $file.Basename
    echo $fn
    adsb-to-parquet $fn
}
