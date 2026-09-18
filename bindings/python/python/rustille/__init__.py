"""Turn pixels into Braille.

``rustille`` renders images as Unicode Braille art and gives you a small
drawing canvas. One Braille character holds a 2x4 grid of dots, so every
character carries eight logical pixels::

            x=0    x=1
    y=0     dot1   dot4      0x01  0x08
    y=1     dot2   dot5      0x02  0x10
    y=2     dot3   dot6      0x04  0x20
    y=3     dot7   dot8      0x40  0x80

    character = U+2800 + mask

Quick start::

    import rustille

    print(rustille.render_file("cat.png", width=80))

All the heavy lifting happens in the Rust core; this module only adds argument
conveniences on top of the native extension.
"""

from __future__ import annotations

from typing import Optional

from ._rustille import (
    Canvas,
    DecodeError,
    RustilleError,
    UnsupportedFormatError,
    __version__,
    braille_char,
    braille_mask,
)
from ._rustille import render_bytes as _render_bytes
from ._rustille import render_file as _render_file
from ._rustille import render_luma as _render_luma
from ._rustille import render_rgb as _render_rgb
from ._rustille import render_rgba as _render_rgba

__all__ = [
    "Canvas",
    "DecodeError",
    "RustilleError",
    "UnsupportedFormatError",
    "__version__",
    "braille_char",
    "braille_mask",
    "render_bytes",
    "render_file",
    "render_luma",
    "render_rgb",
    "render_rgba",
]

#: Width in characters used when neither ``width`` nor ``height`` is given.
DEFAULT_WIDTH = 80
#: Luminance at or above which a dot is drawn.
DEFAULT_THRESHOLD = 128
#: Physical height/width ratio of one terminal character cell.
DEFAULT_CELL_ASPECT_RATIO = 2.0


def _as_bytes(data: object) -> bytes:
    """Accepts any bytes-like object, copying only when it is not ``bytes``."""
    if isinstance(data, bytes):
        return data
    try:
        return bytes(data)  # type: ignore[call-overload]
    except TypeError as error:  # pragma: no cover - defensive
        raise TypeError(
            f"expected a bytes-like object, got {type(data).__name__}"
        ) from error


def render_file(
    path,
    *,
    width: Optional[int] = None,
    height: Optional[int] = None,
    threshold: Optional[int] = None,
    invert: bool = False,
    dither: Optional[str] = None,
    color: Optional[str] = None,
    background: Optional[str] = None,
    fit: Optional[str] = None,
    cell_aspect_ratio: Optional[float] = None,
) -> str:
    """Renders an image file.

    ``width`` and ``height`` are measured in characters. Give one and the other
    is derived from the image's aspect ratio; give neither and the width
    defaults to :data:`DEFAULT_WIDTH`.

    :param path: path to a PNG, JPEG, WebP, BMP, GIF or TIFF file.
    :param threshold: luminance cut-off, ``0``-``255``.
    :param invert: draw dark pixels instead of bright ones.
    :param dither: ``"none"`` or ``"floyd-steinberg"``.
    :param color: ``"none"``, ``"ansi256"`` or ``"truecolor"``.
    :param background: ``"black"``, ``"white"``, ``"#rrggbb"`` or ``"r,g,b"``.
    :param fit: ``"contain"``, ``"fill"`` or ``"stretch"``.
    :param cell_aspect_ratio: height/width ratio of a terminal cell.
    :raises OSError: the file cannot be read.
    :raises DecodeError: the bytes are not a supported image.
    :raises ValueError: an argument is out of range.
    """
    return _render_file(
        path,
        width=width,
        height=height,
        threshold=threshold,
        invert=invert,
        dither=dither,
        color=color,
        background=background,
        fit=fit,
        cell_aspect_ratio=cell_aspect_ratio,
    )


def render_bytes(
    data,
    *,
    width: Optional[int] = None,
    height: Optional[int] = None,
    threshold: Optional[int] = None,
    invert: bool = False,
    dither: Optional[str] = None,
    color: Optional[str] = None,
    background: Optional[str] = None,
    fit: Optional[str] = None,
    cell_aspect_ratio: Optional[float] = None,
) -> str:
    """Renders an encoded image held in memory.

    The format is sniffed from the bytes, so a file name is not needed. See
    :func:`render_file` for the keyword arguments.
    """
    return _render_bytes(
        _as_bytes(data),
        width=width,
        height=height,
        threshold=threshold,
        invert=invert,
        dither=dither,
        color=color,
        background=background,
        fit=fit,
        cell_aspect_ratio=cell_aspect_ratio,
    )


def _raw_renderer(native, channels, kind):
    def render(
        data,
        *,
        width: int,
        height: int,
        output_width: Optional[int] = None,
        output_height: Optional[int] = None,
        threshold: Optional[int] = None,
        invert: bool = False,
        dither: Optional[str] = None,
        color: Optional[str] = None,
        background: Optional[str] = None,
        fit: Optional[str] = None,
        cell_aspect_ratio: Optional[float] = None,
    ) -> str:
        return native(
            _as_bytes(data),
            width=width,
            height=height,
            output_width=output_width,
            output_height=output_height,
            threshold=threshold,
            invert=invert,
            dither=dither,
            color=color,
            background=background,
            fit=fit,
            cell_aspect_ratio=cell_aspect_ratio,
        )

    render.__name__ = f"render_{kind}"
    render.__qualname__ = render.__name__
    render.__doc__ = f"""Renders a tightly packed {kind.upper()} pixel buffer.

    :param data: ``width * height * {channels}`` bytes, row-major.
    :param width: source image width in pixels.
    :param height: source image height in pixels.
    :param output_width: output width in characters.
    :param output_height: output height in characters.

    The remaining keyword arguments are those of :func:`render_file`.
    """
    return render


render_rgba = _raw_renderer(_render_rgba, 4, "rgba")
render_rgb = _raw_renderer(_render_rgb, 3, "rgb")
render_luma = _raw_renderer(_render_luma, 1, "luma")
