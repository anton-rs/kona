package asterisc_test

import (
	"bytes"
	"debug/elf"
	"fmt"
	"io"
	"os"
	"testing"
	"unicode/utf8"

	"github.com/stretchr/testify/require"

	"github.com/ethereum-optimism/asterisc/rvgo/fast"
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

func fullTest(t *testing.T, vmState *fast.VMState, po PreimageOracle, symbols fast.SortedSymbols) (stdOut, stdErr bytes.Buffer) {
	var stdOutBuf, stdErrBuf bytes.Buffer
	instState := fast.NewInstrumentedState(vmState, po, io.MultiWriter(os.Stdout, &stdOutBuf), io.MultiWriter(os.Stderr, &stdErrBuf))

	var lastSym elf.Symbol
	for i := uint64(0); i < 2000_000; i++ {
		sym := symbols.FindSymbol(vmState.PC)

		if sym.Name != lastSym.Name {
			instr := vmState.Instr()
			fmt.Printf("i: %4d  pc: 0x%x  instr: %08x  symbol name: %s size: %d\n", i, vmState.PC, instr, sym.Name, sym.Size)
		}
		lastSym = sym

		if sym.Name == "runtime.throw" {
			throwArg := vmState.Registers[10]
			throwArgLen := vmState.Registers[11]
			if throwArgLen > 1000 {
				throwArgLen = 1000
			}
			x := vmState.Memory.ReadMemoryRange(throwArg, throwArgLen)
			dat, _ := io.ReadAll(x)
			if utf8.Valid(dat) {
				fmt.Printf("THROW! %q\n", string(dat))
			} else {
				fmt.Printf("THROW! %016x: %x\n", throwArg, dat)
			}
			break
		}
		_, err := instState.Step(false)
		require.NoError(t, err, "fast VM must run step")

		if vmState.Exited {
			break
		}
	}

	require.True(t, vmState.Exited, "ran out of steps")
	if vmState.ExitCode != 0 {
		t.Fatalf("failed with exit code %d", vmState.ExitCode)
	}

	return stdOutBuf, stdErrBuf
}
