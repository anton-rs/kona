package cannon_test

import (
	"bytes"
	"crypto/sha256"
	"debug/elf"
	"fmt"
	"io"
	"os"
	"testing"

	"github.com/ethereum-optimism/optimism/cannon/mipsevm"
	"github.com/ethereum/go-ethereum/common"
	"github.com/stretchr/testify/require"

	preimage "github.com/ethereum-optimism/optimism/op-preimage"
)

type PreimageOracle interface {
	Hint(v []byte)
	GetPreimage(k [32]byte) []byte
}

type testOracle struct {
	hint        func(v []byte)
	getPreimage func(k [32]byte) []byte
}

func (t *testOracle) Hint(v []byte) {
	t.hint(v)
}

func (t *testOracle) GetPreimage(k [32]byte) []byte {
	return t.getPreimage(k)
}

var _ PreimageOracle = (*testOracle)(nil)

func rustTestOracle(t *testing.T) (po PreimageOracle, stdOut string, stdErr string) {
	images := make(map[[32]byte][]byte)
	input := []byte("facade facade facade")
	shaHash := sha256.Sum256(input)
	// shaHash[0] = 0x01
	images[preimage.LocalIndexKey(0).PreimageKey()] = input
	images[preimage.LocalIndexKey(1).PreimageKey()] = shaHash[:]
	// CALLDATASIZE
	// PUSH0
	// PUSH0
	// CALLDATACOPY
	// CALLDATASIZE
	// PUSH0
	// RETURN
	images[preimage.LocalIndexKey(2).PreimageKey()] = common.Hex2Bytes("365f5f37365ff3")

	oracle := &testOracle{
		hint: func(v []byte) {
			// no-op
		},
		getPreimage: func(k [32]byte) []byte {
			p, ok := images[k]
			if !ok {
				t.Fatalf("missing pre-image %s", k)
			}
			return p
		},
	}

	return oracle, "", ""
}

func TestSimpleRevm(t *testing.T) {
	elfProgram, err := elf.Open("../../examples/simple-revm/target/mips-unknown-none/release/simple-revm")
	require.NoError(t, err, "open ELF file")

	state, err := mipsevm.LoadELF(elfProgram)
	require.NoError(t, err, "load ELF into state")

	oracle, _, _ := rustTestOracle(t)

	var stdOutBuf, stdErrBuf bytes.Buffer
	us := mipsevm.NewInstrumentedState(state, oracle, io.MultiWriter(&stdOutBuf, os.Stdout), io.MultiWriter(&stdErrBuf, os.Stderr))

	for i := 0; i < 200_000; i++ {
		wit, err := us.Step(true)
		require.NoError(t, err)
		// hack: state isn't exposed in `InstrumentedState`, so we generate the
		// state witness each step and check for the exit condition
		if wit != nil && wit.State[89] == 1 {
			fmt.Printf("exited @ step #%d\n", i)
			break
		}
	}

	require.True(t, state.Exited, "must complete program")
	require.Equal(t, uint8(0), state.ExitCode, "exit with 0")

	require.Equal(t, stdOutBuf.String(), "Booting EVM and checking hash...\nSuccess, hashes matched!\n")
	require.Equal(t, stdErrBuf.String(), "")
}
