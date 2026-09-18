"""Type stubs for the native Rustille extension."""

from pathlib import Path
from typing import Optional, Union

__version__: str

class RustilleError(Exception): ...
class DecodeError(RustilleError): ...
class UnsupportedFormatError(DecodeError): ...

def render_file(
    path: Union[str, Path],
    *,
    width: Optional[int] = ...,
    height: Optional[int] = ...,
    threshold: Optional[int] = ...,
    invert: bool = ...,
    dither: Optional[str] = ...,
    color: Optional[str] = ...,
    background: Optional[str] = ...,
    fit: Optional[str] = ...,
    cell_aspect_ratio: Optional[float] = ...,
) -> str: ...
def render_bytes(
    data: bytes,
    *,
    width: Optional[int] = ...,
    height: Optional[int] = ...,
    threshold: Optional[int] = ...,
    invert: bool = ...,
    dither: Optional[str] = ...,
    color: Optional[str] = ...,
    background: Optional[str] = ...,
    fit: Optional[str] = ...,
    cell_aspect_ratio: Optional[float] = ...,
) -> str: ...
def render_rgba(
    data: bytes,
    *,
    width: int,
    height: int,
    output_width: Optional[int] = ...,
    output_height: Optional[int] = ...,
    threshold: Optional[int] = ...,
    invert: bool = ...,
    dither: Optional[str] = ...,
    color: Optional[str] = ...,
    background: Optional[str] = ...,
    fit: Optional[str] = ...,
    cell_aspect_ratio: Optional[float] = ...,
) -> str: ...
def render_rgb(
    data: bytes,
    *,
    width: int,
    height: int,
    output_width: Optional[int] = ...,
    output_height: Optional[int] = ...,
    threshold: Optional[int] = ...,
    invert: bool = ...,
    dither: Optional[str] = ...,
    color: Optional[str] = ...,
    background: Optional[str] = ...,
    fit: Optional[str] = ...,
    cell_aspect_ratio: Optional[float] = ...,
) -> str: ...
def render_luma(
    data: bytes,
    *,
    width: int,
    height: int,
    output_width: Optional[int] = ...,
    output_height: Optional[int] = ...,
    threshold: Optional[int] = ...,
    invert: bool = ...,
    dither: Optional[str] = ...,
    color: Optional[str] = ...,
    background: Optional[str] = ...,
    fit: Optional[str] = ...,
    cell_aspect_ratio: Optional[float] = ...,
) -> str: ...
def braille_char(mask: int) -> str: ...
def braille_mask(character: str) -> Optional[int]: ...

class Canvas:
    def __init__(self, width: int, height: int) -> None: ...
    @property
    def width(self) -> int: ...
    @property
    def height(self) -> int: ...
    @property
    def cells_width(self) -> int: ...
    @property
    def cells_height(self) -> int: ...
    def set(self, x: int, y: int) -> bool: ...
    def unset(self, x: int, y: int) -> bool: ...
    def toggle(self, x: int, y: int) -> bool: ...
    def get(self, x: int, y: int) -> bool: ...
    def clear(self) -> None: ...
    def fill(self) -> None: ...
    def count(self) -> int: ...
    def line(self, x0: int, y0: int, x1: int, y1: int) -> None: ...
    def rectangle(self, x0: int, y0: int, x1: int, y1: int) -> None: ...
    def filled_rectangle(self, x0: int, y0: int, x1: int, y1: int) -> None: ...
    def circle(self, cx: int, cy: int, radius: int) -> None: ...
    def filled_circle(self, cx: int, cy: int, radius: int) -> None: ...
    def render(self) -> str: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
