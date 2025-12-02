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

void hello_world(void);

void hello_name(const char *name);

/**
 * # Safety
 * It is infact unsafe
 */
enum Status decrypt(struct Cipher cipher, uint8_t *out, uintptr_t size, uintptr_t *actual_size);
