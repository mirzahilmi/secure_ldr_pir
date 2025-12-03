#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum Status {
  Ok = 0,
  Error = -1,
  TooSmall = -2,
} Status;

typedef struct Cipher {
  const char *ciphertext;
  const char *public_key;
  const char *nonce;
} Cipher;

/**
 * # Safety
 * Just believe it
 */
enum Status decrypt(struct Cipher cipher, uint8_t *out, uintptr_t size, uintptr_t *actual_size);
