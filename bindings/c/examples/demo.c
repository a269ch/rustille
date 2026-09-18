/*
 * Minimal C consumer of the Rustille C ABI.
 *
 * Build it with scripts/build-c-example.sh, which generates the header,
 * compiles the library and links this program against it both statically and
 * dynamically.
 *
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <rustille.h>

#define IMAGE_WIDTH 96
#define IMAGE_HEIGHT 96

static void report_error(const char *what, RustilleStatus status) {
  fprintf(stderr, "%s failed: %s", what, rustille_status_message(status));
  char *detail = rustille_last_error_message();
  if (detail != NULL) {
    fprintf(stderr, " (%s)", detail);
    rustille_string_free(detail);
  }
  fputc('\n', stderr);
}

/* Renders a radial gradient straight from an RGBA buffer. */
static int render_pixels(void) {
  uint8_t *pixels = malloc((size_t)IMAGE_WIDTH * IMAGE_HEIGHT * 4);
  if (pixels == NULL) {
    fputs("out of memory\n", stderr);
    return 1;
  }

  for (int y = 0; y < IMAGE_HEIGHT; y++) {
    for (int x = 0; x < IMAGE_WIDTH; x++) {
      double dx = x - IMAGE_WIDTH / 2.0;
      double dy = y - IMAGE_HEIGHT / 2.0;
      double distance = sqrt(dx * dx + dy * dy) / (IMAGE_WIDTH / 2.0);
      double level = 1.0 - distance;
      if (level < 0.0) {
        level = 0.0;
      }
      uint8_t value = (uint8_t)(level * 255.0);
      size_t base = ((size_t)y * IMAGE_WIDTH + (size_t)x) * 4;
      pixels[base + 0] = value;
      pixels[base + 1] = value;
      pixels[base + 2] = value;
      pixels[base + 3] = 255;
    }
  }

  RustilleOptions options;
  RustilleStatus status = rustille_options_init(&options);
  if (status != RUSTILLE_STATUS_OK) {
    report_error("rustille_options_init", status);
    free(pixels);
    return 1;
  }
  options.width = 40;
  options.dither = RUSTILLE_DITHER_FLOYD_STEINBERG;

  char *art = NULL;
  status = rustille_render_rgba(pixels, (size_t)IMAGE_WIDTH * IMAGE_HEIGHT * 4, IMAGE_WIDTH,
                                IMAGE_HEIGHT, &options, &art);
  free(pixels);
  if (status != RUSTILLE_STATUS_OK) {
    report_error("rustille_render_rgba", status);
    return 1;
  }

  puts("-- rustille_render_rgba --");
  puts(art);
  rustille_string_free(art);
  return 0;
}

/* Draws on a canvas. */
static int draw_canvas(void) {
  RustilleCanvas *canvas = rustille_canvas_new(60, 24);
  if (canvas == NULL) {
    report_error("rustille_canvas_new", RUSTILLE_STATUS_INVALID_DIMENSIONS);
    return 1;
  }

  rustille_canvas_rectangle(canvas, 0, 0, 59, 23);
  rustille_canvas_line(canvas, 0, 0, 59, 23);
  rustille_canvas_line(canvas, 59, 0, 0, 23);
  rustille_canvas_circle(canvas, 30, 12, 9);

  char *art = NULL;
  RustilleStatus status = rustille_canvas_render(canvas, &art);
  if (status != RUSTILLE_STATUS_OK) {
    report_error("rustille_canvas_render", status);
    rustille_canvas_free(canvas);
    return 1;
  }

  printf("-- canvas (%u dots set) --\n", rustille_canvas_count(canvas));
  puts(art);
  rustille_string_free(art);
  rustille_canvas_free(canvas);
  return 0;
}

/* The error paths must be reported, never crash. */
static int check_error_handling(void) {
  RustilleOptions options;
  if (rustille_options_init(&options) != RUSTILLE_STATUS_OK) {
    return 1;
  }

  char *art = NULL;
  uint8_t too_short[3] = {0, 0, 0};
  RustilleStatus status =
      rustille_render_rgba(too_short, sizeof too_short, 2, 2, &options, &art);
  if (status != RUSTILLE_STATUS_INVALID_BUFFER_LENGTH) {
    fprintf(stderr, "expected INVALID_BUFFER_LENGTH, got %d\n", (int)status);
    return 1;
  }
  char *detail = rustille_last_error_message();
  if (detail == NULL || strstr(detail, "invalid buffer length") == NULL) {
    fputs("expected a detailed error message\n", stderr);
    rustille_string_free(detail);
    return 1;
  }
  printf("-- error handling --\nbad buffer reported as: %s\n", detail);
  rustille_string_free(detail);

  status = rustille_render_rgba(NULL, 0, 2, 2, &options, &art);
  if (status != RUSTILLE_STATUS_NULL_POINTER) {
    fprintf(stderr, "expected NULL_POINTER, got %d\n", (int)status);
    return 1;
  }

  /* Freeing null and operating on a null canvas must be safe. */
  rustille_string_free(NULL);
  rustille_canvas_free(NULL);
  if (rustille_canvas_width(NULL) != 0) {
    return 1;
  }
  puts("null arguments handled cleanly");
  return 0;
}

int main(void) {
  printf("rustille %s (header %s)\n", rustille_version(), RUSTILLE_VERSION);
  if (strcmp(rustille_version(), RUSTILLE_VERSION) != 0) {
    fputs("header and library versions disagree\n", stderr);
    return 1;
  }

  if (render_pixels() != 0) {
    return 1;
  }
  if (draw_canvas() != 0) {
    return 1;
  }
  if (check_error_handling() != 0) {
    return 1;
  }

  puts("c consumer ok");
  return 0;
}
