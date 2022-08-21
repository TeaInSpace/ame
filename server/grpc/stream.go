package task

import (
	"bytes"
	io "io"
)

// TODO: chunk size needs to be configurable.

// chunkSize determines the size of chunks in ProcessInChunks.
const ChunkSize = 64 * 1024

type (
	// ChunkProcessor is applied to every chunk in ProcessInChunks.
	ChunkProcessor func(data []byte) error
)

// ProcessInChunks calls calls process with each chunk of size chunkSize from data, except
// for the last chunk which might be smaller than chunkSize.
// Any errors found are returned immediately, except for io.EOF which is not considered an error.
// In this case nil will be returned indicating success.
func ProcessInChunks(data *bytes.Buffer, process ChunkProcessor, chunkSize int) error {
	for {
		// TODO: determine if will actually need to make a new byte array for every loop iteration.
		nextChunk := make([]byte, chunkSize)
		_, err := data.Read(nextChunk)
		if err == io.EOF {
			return nil
		}

		if err != nil {
			return err
		}

		err = process(nextChunk)

		if err != nil {
			return err
		}
	}
}
