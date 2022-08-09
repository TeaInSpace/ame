package auth

import (
	"context"
	"fmt"
	"os"

	grpc_auth "github.com/grpc-ecosystem/go-grpc-middleware/auth"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// AmeAuthTokenEnvVarKey contains the key for the environment variable that the AME server will look at
// to find an authentication token to validate requests against.
const AmeAuthTokenEnvVarKey = "AME_AUTH_TOKEN"

// EnvAuthenticator reads the AME_AUTH_TOKEN environment variable at the time of
// execution and returns an AuthFunc which will validate requests based on that value
// , meanin the AuthFunc will not read from the environment while validating requests.
// No changes are made to the supplied context and a nil error indicates success and
// a non nil error indicates failure.
func EnvAuthenticator() (grpc_auth.AuthFunc, error) {
	token := os.Getenv(AmeAuthTokenEnvVarKey)
	if token == "" {
		return nil, fmt.Errorf("no token was found in the environment variable %v", AmeAuthTokenEnvVarKey)
	}

	return func(ctx context.Context) (context.Context, error) {
		bearer, err := grpc_auth.AuthFromMD(ctx, "bearer")
		if err != nil {
			return ctx, err
		}

		if bearer != token {
			return ctx, status.Errorf(codes.Unauthenticated, "The bearer token did match the token in the server configuration")
		}

		return ctx, nil
	}, nil
}
