# WinForms target window for tests/windows_actions.rs.
# Writes ready.json (control screen rects), text.txt (TextBox contents) and
# events.txt (one mouse event per line) under -Out.
param([Parameter(Mandatory)][string]$Out)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace SqyreFx -Name Dpi -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[SqyreFx.Dpi]::SetProcessDPIAware() | Out-Null

$events = Join-Path $Out "events.txt"
$textFile = Join-Path $Out "text.txt"
function Log-Event([string]$line) { [IO.File]::AppendAllText($events, "$line`n") }

$form = New-Object Windows.Forms.Form
$form.Text = "Sqyre Action Fixture"
$form.StartPosition = "Manual"
$form.Location = New-Object Drawing.Point(60, 60)
$form.ClientSize = New-Object Drawing.Size(660, 420)
$form.BackColor = [Drawing.Color]::White
$form.TopMost = $true

$label = New-Object Windows.Forms.Label
$label.Text = "SQYRE"
$label.Font = New-Object Drawing.Font("Arial", 40, [Drawing.FontStyle]::Bold)
$label.ForeColor = [Drawing.Color]::Black
$label.AutoSize = $true
$label.Location = New-Object Drawing.Point(20, 20)

$swatch = New-Object Windows.Forms.Panel
$swatch.BackColor = [Drawing.Color]::FromArgb(0x12, 0xEF, 0x34)
$swatch.Location = New-Object Drawing.Point(420, 20)
$swatch.Size = New-Object Drawing.Size(60, 60)

# Distinctive pattern for Image Search; the test captures it as the template.
$bmp = New-Object Drawing.Bitmap(64, 64)
$g = [Drawing.Graphics]::FromImage($bmp)
$g.Clear([Drawing.Color]::Navy)
$g.FillRectangle([Drawing.Brushes]::OrangeRed, 6, 6, 30, 22)
$g.FillEllipse([Drawing.Brushes]::Gold, 26, 26, 32, 32)
$g.DrawLine((New-Object Drawing.Pen([Drawing.Color]::White, 3)), 0, 63, 63, 0)
$g.Dispose()
$pattern = New-Object Windows.Forms.PictureBox
$pattern.Image = $bmp
$pattern.Location = New-Object Drawing.Point(520, 20)
$pattern.Size = New-Object Drawing.Size(64, 64)

$text = New-Object Windows.Forms.TextBox
$text.Multiline = $true
$text.AcceptsReturn = $true
$text.Font = New-Object Drawing.Font("Consolas", 12)
$text.Location = New-Object Drawing.Point(20, 120)
$text.Size = New-Object Drawing.Size(360, 150)

$button = New-Object Windows.Forms.Button
$button.Text = "Click target"
$button.Location = New-Object Drawing.Point(420, 120)
$button.Size = New-Object Drawing.Size(180, 60)
$button.add_MouseUp({ param($s, $e) Log-Event "button:$($e.Button)" })
$button.add_MouseWheel({ param($s, $e) Log-Event "wheel:$([Math]::Sign($e.Delta))" })

$cells = @()
foreach ($i in 0..1) {
    $cell = New-Object Windows.Forms.Panel
    $cell.BackColor = [Drawing.Color]::LightGray
    $cell.BorderStyle = "FixedSingle"
    $cell.Location = New-Object Drawing.Point((420 + 90 * $i), 220)
    $cell.Size = New-Object Drawing.Size(90, 50)
    $cell.Tag = "cell$($i + 1)"
    $cell.add_MouseUp({ param($s, $e) Log-Event "$($s.Tag):$($e.Button)" })
    $cells += $cell
}

$form.Controls.AddRange(@($label, $swatch, $pattern, $text, $button) + $cells)

$timer = New-Object Windows.Forms.Timer
$timer.Interval = 100
$script:last = $null
$timer.add_Tick({
    if ($text.Text -ne $script:last) {
        $script:last = $text.Text
        [IO.File]::WriteAllText($textFile, $text.Text)
    }
})

function Rect($c) {
    $r = $c.RectangleToScreen($c.ClientRectangle)
    @{ x = $r.X; y = $r.Y; w = $r.Width; h = $r.Height }
}
$form.add_Shown({
    $form.Activate()
    $ready = @{
        exe = (Get-Process -Id $PID).Path
        title = $form.Text
        form = Rect $form
        label = Rect $label
        swatch = Rect $swatch
        pattern = Rect $pattern
        text = Rect $text
        button = Rect $button
        grid = @{ x = (Rect $cells[0]).x; y = (Rect $cells[0]).y; w = 180; h = 50 }
    }
    [IO.File]::WriteAllText((Join-Path $Out "ready.json"), ($ready | ConvertTo-Json -Compress))
    $timer.Start()
})
[Windows.Forms.Application]::Run($form)
