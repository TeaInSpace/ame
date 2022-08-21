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
		header string
		name   string
		token  string
		want   bool
	}{
		{
			header: "authorization",
			name:   "Correct token",
			token:  token,
			want:   true,
		},
		{
			header: "authorization",
			name:   "Incorrect token",
			token:  "sfsdf",
			want:   false,
		},
		{
			header: "authorization",
			name:   "Empty token",
			token:  "",
			want:   false,
		},
		{
			header: "authoriion",
			name:   "Bad metadata",
			token:  token,
			want:   false,
		},
	}

	for _, tc := range testCases {
		t.Run(fmt.Sprintf("Test authentication with %v", tc.name), func(t *testing.T) {
			ctx := context.Background()
			ctx = metadata.NewIncomingContext(ctx, metadata.MD{tc.header: []string{"Bearer " + tc.token}})
			_, err := authenticator(ctx)

			if (err != nil) && tc.want {
				t.Errorf("authenicator(ctx), returned a non nil error %v but expected a nil error", err)
			} else if (err == nil) && !tc.want {
				t.Error("authenicator(ctx), returned a nil error but expected a non nil error")
			}
		})
	}
}

func TestMissingTokenEnvVar(t *testing.T) {
	err := os.Unsetenv(AmeAuthTokenEnvVarKey)
	if err != nil {
		t.Fatal(err)
	}

	_, err = EnvAuthenticator()
	if err == nil {
		t.Errorf("expected an error due to no authenticatin token being set in the environemnt")
	}
}
