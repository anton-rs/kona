package asterisc_test

import (
	"debug/elf"
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/ethereum-optimism/asterisc/rvgo/fast"
)

func TestMinimal(t *testing.T) {
	programELF, err := elf.Open("../bin/asterisc/minimal")
	require.NoError(t, err)
	defer programELF.Close()

	po, _, _ := rustTestOracle(t)

	symbols, err := fast.Symbols(programELF)
	require.NoError(t, err)

	vmState, err := fast.LoadELF(programELF)
	require.NoError(t, err, "must load test suite ELF binary")

	stdOutBuf, stdErrBuf := fullTest(t, vmState, po, symbols)
	require.Equal(t, "Hello, world!\n", stdOutBuf.String(), "stdout says hello")
	require.Equal(t, "", stdErrBuf.String(), "stderr silent")
}
