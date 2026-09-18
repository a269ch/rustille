"""Smoke and integration tests for the Rustille Python package.

These run against an installed wheel (or an editable `maturin develop`
build), so they exercise the real native extension rather than the Rust
sources.
"""

from __future__ import annotations

import struct
import zlib

import pytest

import rustille


def make_png(width: int, height: int, pixel) -> bytes:
    """Encodes an RGBA PNG without pulling in an image library."""
    raw = bytearray()
    for y in range(height):
        raw.append(0)  # filter: none
        for x in range(width):
            raw.extend(pixel(x, y))

    def chunk(kind: bytes, payload: bytes) -> bytes:
        return (
            struct.pack(">I", len(payload))
            + kind
            + payload
            + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
        )

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


@pytest.fixture
def checkerboard_png() -> bytes:
    def pixel(x: int, y: int):
        value = 255 if (x // 4 + y // 4) % 2 == 0 else 0
        return (value, value, value, 255)

    return make_png(32, 32, pixel)


def is_braille(text: str) -> bool:
    return bool(text) and all(c == "\n" or 0x2800 <= ord(c) <= 0x28FF for c in text)


def test_version_is_exposed():
    assert rustille.__version__.count(".") == 2


def test_render_bytes(checkerboard_png):
    art = rustille.render_bytes(checkerboard_png, width=16)
    assert is_braille(art)
    assert all(len(line) == 16 for line in art.splitlines())


def test_render_file(tmp_path, checkerboard_png):
    path = tmp_path / "checkerboard.png"
    path.write_bytes(checkerboard_png)

    from_path = rustille.render_file(path, width=16)
    from_str = rustille.render_file(str(path), width=16)
    assert from_path == from_str == rustille.render_bytes(checkerboard_png, width=16)


def test_render_rgba_matches_the_documented_signature():
    width = height = 16
    rgba = bytes([255, 255, 255, 255] * (width * height))
    art = rustille.render_rgba(rgba, width=width, height=height, output_width=8)
    assert art.splitlines()[0] == "⣿" * 8


def test_render_rgb_and_luma_agree_with_rgba():
    width = height = 8
    luma = bytes(range(0, 255, 4))[: width * height]
    rgb = bytes(v for value in luma for v in (value, value, value))
    rgba = bytes(v for value in luma for v in (value, value, value, 255))

    kwargs = dict(width=width, height=height, output_width=4, fit="stretch")
    assert (
        rustille.render_luma(luma, **kwargs)
        == rustille.render_rgb(rgb, **kwargs)
        == rustille.render_rgba(rgba, **kwargs)
    )


def test_bytes_like_inputs_are_accepted(checkerboard_png):
    expected = rustille.render_bytes(checkerboard_png, width=8)
    assert rustille.render_bytes(bytearray(checkerboard_png), width=8) == expected
    assert rustille.render_bytes(memoryview(checkerboard_png), width=8) == expected


def test_options_change_the_output(checkerboard_png):
    plain = rustille.render_bytes(checkerboard_png, width=8)
    assert rustille.render_bytes(checkerboard_png, width=8, invert=True) != plain
    # The fixture is pure black and white, so only threshold 0 ("everything is
    # a dot") changes which dots light up.
    all_lit = rustille.render_bytes(checkerboard_png, width=8, threshold=0)
    assert all_lit != plain
    assert set(all_lit) == {"\u28ff", "\n"}

    dithered = rustille.render_bytes(
        checkerboard_png, width=8, dither="floyd-steinberg"
    )
    assert is_braille(dithered)

    colored = rustille.render_bytes(checkerboard_png, width=8, color="truecolor")
    assert "\x1b[38;2;" in colored
    assert "\x1b[" not in plain

    assert len(rustille.render_bytes(checkerboard_png, height=4).splitlines()) == 4
    stretched = rustille.render_bytes(
        checkerboard_png, width=8, height=8, fit="stretch"
    )
    assert len(stretched.splitlines()) == 8


def test_background_is_composited():
    transparent = make_png(8, 8, lambda x, y: (255, 255, 255, 0))
    on_black = rustille.render_bytes(transparent, width=2, height=1, fit="stretch")
    on_white = rustille.render_bytes(
        transparent, width=2, height=1, fit="stretch", background="white"
    )
    assert on_black == "⠀⠀"
    assert on_white == "⣿⣿"


def test_errors_are_pythonic(tmp_path):
    with pytest.raises(OSError):
        rustille.render_file(tmp_path / "missing.png")

    with pytest.raises(rustille.DecodeError):
        rustille.render_bytes(b"not an image")

    with pytest.raises(ValueError):
        rustille.render_rgba(b"\x00\x00\x00", width=2, height=2)

    with pytest.raises(ValueError):
        rustille.render_bytes(make_png(4, 4, lambda x, y: (0, 0, 0, 255)), width=0)

    with pytest.raises(ValueError):
        rustille.render_bytes(b"", dither="atkinson")

    assert issubclass(rustille.DecodeError, rustille.RustilleError)
    assert issubclass(rustille.UnsupportedFormatError, rustille.DecodeError)


def test_canvas():
    canvas = rustille.Canvas(100, 50)
    assert (canvas.width, canvas.height) == (100, 50)
    assert (canvas.cells_width, canvas.cells_height) == (50, 13)

    assert canvas.set(1, 1)
    assert canvas.get(1, 1)
    assert not canvas.set(1000, 1000)
    assert canvas.count() == 1

    canvas.unset(1, 1)
    assert not canvas.get(1, 1)

    canvas.toggle(0, 0)
    assert canvas.get(0, 0)

    canvas.clear()
    canvas.line(0, 0, 99, 49)
    canvas.rectangle(0, 0, 99, 49)
    canvas.filled_rectangle(10, 10, 20, 20)
    canvas.circle(50, 25, 10)
    canvas.filled_circle(50, 25, 3)
    assert canvas.count() > 0

    rendered = canvas.render()
    assert rendered == str(canvas)
    assert len(rendered.splitlines()) == 13
    assert is_braille(rendered)
    assert "Canvas(width=100" in repr(canvas)

    canvas.fill()
    assert canvas.count() == 100 * 50

    with pytest.raises(ValueError):
        rustille.Canvas(2**32 - 1, 2**32 - 1)


def test_braille_helpers():
    assert rustille.braille_char(0) == "⠀"
    assert rustille.braille_char(255) == "⣿"
    assert rustille.braille_mask("⣿") == 255
    assert rustille.braille_mask("a") is None
    assert rustille.braille_mask("ab") is None
    for mask in range(256):
        assert rustille.braille_mask(rustille.braille_char(mask)) == mask
