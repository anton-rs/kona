package cannon_test

import (
	"bytes"
	"debug/elf"
	"fmt"
	"io"
	"os"
	"testing"

	"github.com/ethereum-optimism/optimism/cannon/mipsevm"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/crypto"
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
	images[preimage.LocalIndexKey(0).PreimageKey()] = crypto.Keccak256([]byte("started!"))

	oracle := &testOracle{
		hint: func(v []byte) {
			// no-op
		},
		getPreimage: func(k [32]byte) []byte {
			fmt.Println("getPreimage", common.Bytes2Hex(k[:]))
			p, ok := images[k]
			if !ok {
				t.Fatalf("missing pre-image %s", k)
			}
			return p
		},
	}

	return oracle, "", ""
}

func TestRust(t *testing.T) {
	elfProgram, err := elf.Open("../target/mips-unknown-none/release/simple-revm")
	require.NoError(t, err, "open ELF file")

	state, err := mipsevm.LoadELF(elfProgram)
	require.NoError(t, err, "load ELF into state")

	// err = PatchGo(elfProgram, state)
	// require.NoError(t, err, "apply Go runtime patches")
	// require.NoError(t, PatchStack(state), "add initial stack")

	oracle, _, _ := rustTestOracle(t)

	var stdOutBuf, stdErrBuf bytes.Buffer
	us := mipsevm.NewInstrumentedState(state, oracle, io.MultiWriter(&stdOutBuf, os.Stdout), io.MultiWriter(&stdErrBuf, os.Stderr))

	for i := 0; i < 400_000; i++ {
		wit, err := us.Step(false)
		require.NoError(t, err)
		if wit != nil && wit.State[90] == 1 {
			fmt.Printf("exited @ %d\n", 0)
			break
		}
	}

	require.True(t, state.Exited, "must complete program")
	require.Equal(t, uint8(0), state.ExitCode, "exit with 0")

	// require.Equal(t, "hello world!\n", stdOutBuf.String(), "stdout says hello")
	// require.Equal(t, "", stdErrBuf.String(), "stderr silent")
}
