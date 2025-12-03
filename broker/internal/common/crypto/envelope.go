package crypto

/*
#cgo LDFLAGS: -L. -lcrypto
#include "crypto_bindings.h"
#include <stdlib.h>
*/
import "C"
import (
	"unsafe"
)

func EnvelopeUnseal(ciphertext, nonce, publicKey string) (string, error) {
	_ciphertext := C.CString(ciphertext)
	defer C.free(unsafe.Pointer(_ciphertext))
	_nonce := C.CString(nonce)
	defer C.free(unsafe.Pointer(_nonce))
	_publicKey := C.CString(publicKey)
	defer C.free(unsafe.Pointer(_publicKey))

	cipher := C.Cipher{
		ciphertext: _ciphertext,
		nonce:      _nonce,
		public_key: _publicKey,
	}
	actualSize := new(C.size_t)
	defer C.free(unsafe.Pointer(actualSize))

	// probably should check exit code
	C.decrypt(cipher, nil, 0, actualSize)

	buf := make([]byte, *actualSize)
	C.decrypt(
		cipher,
		(*C.uchar)(unsafe.Pointer(&buf[0])),
		C.size_t(len(buf)),
		actualSize,
	)

	return string(buf), nil
}
