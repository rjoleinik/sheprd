# Synthetic attachment preview inputs

`Review report.pdf` is a minimal one-page PDF with a title and three bars,
written directly using standard PDF objects and a cross-reference table.
It contains no external resources or personal data.

`Video walkthrough.mp4` is a one-second H.264 color test pattern:

```sh
ffmpeg -f lavfi -i 'testsrc2=size=480x270:rate=12' -t 1 \
  -c:v libx264 -pix_fmt yuv420p -movflags +faststart 'Video walkthrough.mp4'
```

Used by the `files` native chat fixture and the opt-in macOS Quick Look test.
