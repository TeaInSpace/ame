package auth

import (
	"context"
	"fmt"
	"os"
	"testing"

	"google.golang.org/grpc/metadata"
)

func TestAuthenticateAgainstTokenInEnvironment(t *testing.T) {
	token := "mytoken"
	os.Setenv(AmeAuthTokenEnvVarKey, token)
	authenticator, err := EnvAuthenticator()
	if err != nil {
		t.Error(err)
	}

	// To verify that the authenticator is not reading from
	// the environment after it's creation, we unset the
	// environment variable.
	err = os.Unsetenv(AmeAuthTokenEnvVarKey)
	if err != nil {
		t.Error(err)
	}

	testCases := []struct {
		name  string
		token string
		want  bool
	}{
		{
			name:  "Correct token",
			token: token,
			want:  true,
		},
		{
			name:  "Incorrect token",
			token: "sfsdf",
			want:  false,
		},
		{
			name:  "Empty token",
			token: "",
			want:  false,
		},
	}

	for _, tc := range testCases {
		t.Run(fmt.Sprintf("Test authentication with %v", tc.name), func(t *testing.T) {
			ctx := context.Background()
			ctx = metadata.NewIncomingContext(ctx, metadata.MD{"authorization": []string{"Bearer " + tc.token}})
			_, err := authenticator(ctx)

			if (err != nil) && tc.want {
				t.Errorf("authenicator(ctx), returned a non nil error %v but expected a nil error", err)
			} else if (err == nil) && !tc.want {
				t.Error("authenicator(ctx), returned a nil error but expected a non nil error")
			}
		})
	}
}
