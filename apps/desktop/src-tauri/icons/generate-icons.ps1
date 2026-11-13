# Generates the Elyra application icon set from one vector-like composition.
# Run with: powershell -ExecutionPolicy Bypass -File .\generate-icons.ps1
# Requires Windows PowerShell 5.1+ (System.Drawing).

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$directory = Split-Path -Parent $MyInvocation.MyCommand.Path
$masterSize = 1024

function New-ElyraMaster {
    param([int]$Size)

    $bitmap = New-Object System.Drawing.Bitmap($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.Clear([System.Drawing.Color]::Transparent)

    $radius = [single]($Size * 0.225)
    $bounds = New-Object System.Drawing.RectangleF(0, 0, $Size, $Size)
    $shape = New-Object System.Drawing.Drawing2D.GraphicsPath
    $diameter = $radius * 2
    $shape.AddArc(0, 0, $diameter, $diameter, 180, 90)
    $shape.AddArc($Size - $diameter, 0, $diameter, $diameter, 270, 90)
    $shape.AddArc($Size - $diameter, $Size - $diameter, $diameter, $diameter, 0, 90)
    $shape.AddArc(0, $Size - $diameter, $diameter, $diameter, 90, 90)
    $shape.CloseFigure()

    $background = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $bounds,
        [System.Drawing.Color]::FromArgb(255, 77, 122, 97),
        [System.Drawing.Color]::FromArgb(255, 48, 79, 62),
        52
    )
    $graphics.FillPath($background, $shape)

    # A quiet top-left sheen keeps the flat square from looking heavy.
    $sheenBounds = New-Object System.Drawing.RectangleF(0, 0, $Size, $Size)
    $sheen = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $sheenBounds,
        [System.Drawing.Color]::FromArgb(46, 255, 255, 255),
        [System.Drawing.Color]::FromArgb(0, 255, 255, 255),
        120
    )
    $graphics.SetClip($shape)
    $graphics.FillRectangle($sheen, $sheenBounds)
    $graphics.ResetClip()

    # The mark is the same lowercase serif e used on the website and in-app.
    $family = New-Object System.Drawing.FontFamily("Georgia")
    $style = [System.Drawing.FontStyle]::Bold -bor [System.Drawing.FontStyle]::Italic
    $glyph = New-Object System.Drawing.Drawing2D.GraphicsPath
    $format = [System.Drawing.StringFormat]::GenericDefault
    $glyph.AddString("e", $family, [int]$style, 1000, (New-Object System.Drawing.PointF(0, 0)), $format)

    $glyphBounds = $glyph.GetBounds()
    $targetWidth = $Size * 0.47
    $targetHeight = $Size * 0.50
    $scale = [Math]::Min($targetWidth / $glyphBounds.Width, $targetHeight / $glyphBounds.Height)

    $matrix = New-Object System.Drawing.Drawing2D.Matrix
    $matrix.Translate(-$glyphBounds.X, -$glyphBounds.Y)
    $matrix.Scale([single]$scale, [single]$scale, [System.Drawing.Drawing2D.MatrixOrder]::Append)
    $newWidth = $glyphBounds.Width * $scale
    $newHeight = $glyphBounds.Height * $scale
    $matrix.Translate(
        [single](($Size - $newWidth) / 2),
        [single](($Size - $newHeight) / 2 + ($Size * 0.012)),
        [System.Drawing.Drawing2D.MatrixOrder]::Append
    )
    $glyph.Transform($matrix)

    $ink = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 248, 251, 244))
    $graphics.FillPath($ink, $glyph)

    $ink.Dispose()
    $glyph.Dispose()
    $format.Dispose()
    $family.Dispose()
    $sheen.Dispose()
    $background.Dispose()
    $shape.Dispose()
    $graphics.Dispose()

    return $bitmap
}

function Save-Png {
    param(
        [System.Drawing.Bitmap]$Source,
        [string]$Path,
        [int]$Size
    )

    $bitmap = New-Object System.Drawing.Bitmap($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.Clear([System.Drawing.Color]::Transparent)
    $graphics.DrawImage($Source, 0, 0, $Size, $Size)
    $graphics.Dispose()
    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
}

function Get-PngBytes {
    param([System.Drawing.Bitmap]$Source, [int]$Size)
    $stream = New-Object System.IO.MemoryStream
    $copy = New-Object System.Drawing.Bitmap($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($copy)
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.Clear([System.Drawing.Color]::Transparent)
    $graphics.DrawImage($Source, 0, 0, $Size, $Size)
    $graphics.Dispose()
    $copy.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    $copy.Dispose()
    $bytes = $stream.ToArray()
    $stream.Dispose()
    Write-Output -NoEnumerate $bytes
}

function Write-Ico {
    param([System.Drawing.Bitmap]$Source, [string]$Path)

    $sizes = @(16, 24, 32, 48, 64, 128, 256)
    $images = @()
    foreach ($size in $sizes) {
        $images += ,@($size, (Get-PngBytes -Source $Source -Size $size))
    }

    $stream = New-Object System.IO.MemoryStream
    $writer = New-Object System.IO.BinaryWriter($stream)
    $writer.Write([UInt16]0)
    $writer.Write([UInt16]1)
    $writer.Write([UInt16]$images.Count)

    $offset = 6 + (16 * $images.Count)
    foreach ($image in $images) {
        $size = $image[0]
        $bytes = $image[1]
        $writer.Write([byte]$(if ($size -ge 256) { 0 } else { $size }))
        $writer.Write([byte]$(if ($size -ge 256) { 0 } else { $size }))
        $writer.Write([byte]0)
        $writer.Write([byte]0)
        $writer.Write([UInt16]1)
        $writer.Write([UInt16]32)
        $writer.Write([UInt32]$bytes.Length)
        $writer.Write([UInt32]$offset)
        $offset += $bytes.Length
    }

    foreach ($image in $images) {
        $bytes = [byte[]]$image[1]
        $writer.Write($bytes, 0, $bytes.Length)
    }

    $writer.Flush()
    [System.IO.File]::WriteAllBytes($Path, $stream.ToArray())
    $writer.Dispose()
    $stream.Dispose()
}

function Write-BE32 {
    param([System.IO.BinaryWriter]$Writer, [UInt32]$Value)
    $bytes = [BitConverter]::GetBytes($Value)
    [Array]::Reverse($bytes)
    $Writer.Write($bytes, 0, 4)
}

function Write-Icns {
    param([System.Drawing.Bitmap]$Source, [string]$Path)

    $chunks = @(
        @("icp4", 16),
        @("icp5", 32),
        @("icp6", 64),
        @("ic07", 128),
        @("ic08", 256),
        @("ic09", 512),
        @("ic10", 1024)
    )

    $body = New-Object System.IO.MemoryStream
    $writer = New-Object System.IO.BinaryWriter($body)
    foreach ($chunk in $chunks) {
        $type = [System.Text.Encoding]::ASCII.GetBytes($chunk[0])
        $bytes = [byte[]](Get-PngBytes -Source $Source -Size $chunk[1])
        $writer.Write($type, 0, $type.Length)
        Write-BE32 -Writer $writer -Value ([UInt32]($bytes.Length + 8))
        $writer.Write($bytes, 0, $bytes.Length)
    }
    $writer.Flush()
    $payload = $body.ToArray()
    $writer.Dispose()
    $body.Dispose()

    $stream = New-Object System.IO.MemoryStream
    $writer = New-Object System.IO.BinaryWriter($stream)
    $magic = [System.Text.Encoding]::ASCII.GetBytes("icns")
    $writer.Write($magic, 0, $magic.Length)
    Write-BE32 -Writer $writer -Value ([UInt32]($payload.Length + 8))
    $writer.Write($payload, 0, $payload.Length)
    $writer.Flush()
    [System.IO.File]::WriteAllBytes($Path, $stream.ToArray())
    $writer.Dispose()
    $stream.Dispose()
}

$master = New-ElyraMaster -Size $masterSize

Save-Png -Source $master -Path (Join-Path $directory "icon-source.png") -Size 1024
Save-Png -Source $master -Path (Join-Path $directory "icon.png") -Size 512
Save-Png -Source $master -Path (Join-Path $directory "32x32.png") -Size 32
Save-Png -Source $master -Path (Join-Path $directory "128x128.png") -Size 128
Save-Png -Source $master -Path (Join-Path $directory "128x128@2x.png") -Size 256

$storeSizes = @(30, 44, 71, 89, 107, 142, 150, 284, 310)
foreach ($size in $storeSizes) {
    Save-Png -Source $master -Path (Join-Path $directory "Square${size}x${size}Logo.png") -Size $size
}
Save-Png -Source $master -Path (Join-Path $directory "StoreLogo.png") -Size 50

Write-Ico -Source $master -Path (Join-Path $directory "icon.ico")
Write-Icns -Source $master -Path (Join-Path $directory "icon.icns")

$master.Dispose()
Write-Host "Generated Elyra icon set in $directory"
