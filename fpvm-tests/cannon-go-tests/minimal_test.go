package cannon_test

import (
	"bytes"
	"debug/elf"
	"fmt"
	"io"
	"os"
	"testing"

	"github.com/ethereum-optimism/optimism/cannon/mipsevm"
	"github.com/stretchr/testify/require"
)

func TestMinimal(t *testing.T) {
	elfProgram, err := elf.Open("../../examples/minimal/target/mips-unknown-none/release/minimal")
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

	require.Equal(t, "Hello, world!\n", stdOutBuf.String(), "stdout says hello")
	require.Equal(t, "", stdErrBuf.String(), "stderr silent")
}
