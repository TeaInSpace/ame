package filescanner

import "context"

type Transporter interface {
	TransportDir(ctx context.Context, data []byte) error
}
