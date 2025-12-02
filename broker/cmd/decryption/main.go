package main

/*
#cgo LDFLAGS: -L../../../crypto/target/release -lcrypto
#include "../../crypto_bindings.h"
#include <stdlib.h>
*/
import "C"
import (
	"fmt"
	"unsafe"
)

func main() {
	ciphertext := C.CString("+cF4V6z9OnbolyT+5Z2DrHuox+PRuqwX/TaUPIVryUqjRTm0Pc9/GXlbNJG0XwnyRsXr1fUsx6XlwKDwGsTettEGie2ld7U58EI4/pFzq20IIO6w4jRxgkDkn9iix8lfw1J9Ew==")
	defer C.free(unsafe.Pointer(ciphertext))
	publicKey := C.CString("25ru/9JtiE5b0cUHHDHoRqNlnxRBlLhQKoQ5vxX3tE8=")
	defer C.free(unsafe.Pointer(publicKey))
	nonce := C.CString("6wg6L5GxAAABmrpvISEADg==")
	defer C.free(unsafe.Pointer(nonce))

	cipher := C.Cipher{
		ciphertext: ciphertext,
		public_key: publicKey,
		nonce:      nonce,
	}

	actualSize := new(C.size_t)
	// probably should check exit code
	C.decrypt(cipher, nil, 0, actualSize)
	buf := make([]byte, *actualSize)
	C.decrypt(
		cipher,
		(*C.uchar)(unsafe.Pointer(&buf[0])),
		C.size_t(len(buf)),
		actualSize,
	)

	fmt.Println(string(buf))
}
