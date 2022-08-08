package storage

import (
	"bytes"
	"context"
	"fmt"
	"io/ioutil"
	"strings"

	"golang.org/x/sync/errgroup"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	"github.com/aws/aws-sdk-go-v2/service/s3/types"
)

type Storage interface {
	StoreFile(ctx context.Context, file ProjectFile) error
	DownloadFiles(ctx context.Context, path string) ([]ProjectFile, error)
	PrepareStorage(ctx context.Context) error
	ClearStorage(ctx context.Context) error
}

type ProjectFile struct {
	Path string
	Data []byte
}

type s3Storage struct {
	s3Client   s3.Client
	bucketName string
}

func NewS3Storage(s3Client s3.Client, bucketName string) Storage {
	return &s3Storage{s3Client, bucketName}
}

func (s *s3Storage) PrepareStorage(ctx context.Context) error {
	res, err := s.s3Client.ListBuckets(ctx, &s3.ListBucketsInput{})
	if err != nil {
		return err
	}

	for _, bucket := range res.Buckets {
		if *bucket.Name == s.bucketName {
			return nil
		}
	}

	_, err = s.s3Client.CreateBucket(ctx, &s3.CreateBucketInput{Bucket: aws.String(s.bucketName)})
	if err != nil {
		return err
	}

	return nil
}

func (s *s3Storage) ClearStorage(ctx context.Context) error {
	contents, err := s.listStoredFiles(ctx)
	if err != nil {
		return err
	}

	objectsToDelete := []types.ObjectIdentifier{}
	for _, c := range contents {
		objectsToDelete = append(objectsToDelete, types.ObjectIdentifier{Key: aws.String(c)})
	}

	if len(objectsToDelete) > 0 {
		_, err = s.s3Client.DeleteObjects(ctx, &s3.DeleteObjectsInput{
			Delete: &types.Delete{
				Objects: objectsToDelete,
			},
			Bucket: aws.String(s.bucketName),
		})

		if err != nil {
			return err
		}
	}

	_, err = s.s3Client.DeleteBucket(ctx, &s3.DeleteBucketInput{Bucket: aws.String(s.bucketName)})
	if err != nil {
		return err
	}

	return nil
}

func (s *s3Storage) StoreFile(ctx context.Context, projectFile ProjectFile) error {
	output, err := s.s3Client.PutObject(ctx, &s3.PutObjectInput{
		Bucket: aws.String(s.bucketName),
		Key:    aws.String(projectFile.Path),
		Body:   bytes.NewReader(projectFile.Data),
	})
	if err != nil {
		return fmt.Errorf("Got err: %s from S3 client with output %+v", err, output)
	}

	return nil
}

func (s *s3Storage) listStoredFilesWithPrefix(ctx context.Context, prefix string) ([]string, error) {
	objects, err := listBucketContents(ctx, &s.s3Client, s.bucketName, "")
	if err != nil {
		return nil, err
	}

	paths := []string{}
	for _, o := range objects {
		paths = append(paths, *o.Key)
	}

	return paths, err
}

func (s *s3Storage) listStoredFiles(ctx context.Context) ([]string, error) {
	return s.listStoredFilesWithPrefix(ctx, "")
}

func listBucketContents(ctx context.Context, s3Client *s3.Client, bucketName string, prefix string) ([]types.Object, error) {
	paginator := s3.NewListObjectsV2Paginator(s3Client, &s3.ListObjectsV2Input{
		Bucket: aws.String(bucketName),
		Prefix: aws.String(prefix),
	})

	contents := []types.Object{}
	for {
		if !paginator.HasMorePages() {
			break
		}

		out, err := paginator.NextPage(ctx)
		if err != nil {
			return nil, err
		}

		contents = append(contents, out.Contents...)
	}

	return contents, nil
}

func (s *s3Storage) DownloadFiles(ctx context.Context, projectDir string) ([]ProjectFile, error) {
	contents, err := s.listStoredFilesWithPrefix(ctx, projectDir)
	if err != nil {
		return nil, err
	}

	files := make([]ProjectFile, len(contents))
	eGroup, ctx := errgroup.WithContext(ctx)

	for i, c := range contents {
		// Declaring these variables within the loop ensures that
		// each goroutine can reference its own set of variables, otherwise
		// we would create a datarace where they are all referencing the same
		// variables from the for loop iterator.
		filePath := c
		goRoutineIndex := i

		eGroup.Go(func() error {
			output, err := s.s3Client.GetObject(ctx, &s3.GetObjectInput{
				Bucket: aws.String(s.bucketName),
				Key:    aws.String(filePath),
			})
			if err != nil {
				return err
			}

			defer output.Body.Close()

			data, err := ioutil.ReadAll(output.Body)
			if err != nil {
				return err
			}
			parentDirSplit := strings.Split(filePath, projectDir+"/")

			files[goRoutineIndex] = ProjectFile{parentDirSplit[len(parentDirSplit)-1], data}
			return nil
		})
	}

	err = eGroup.Wait()
	if err != nil {
		return nil, err
	}

	return files, nil
}

func CreateS3Client(ctx context.Context, endpoint string, region string, overrider ...func(*s3.Options)) (*s3.Client, error) {
	// We need to ensure that all requests resolve to the endpoint where minio is running.
	// This does not match the normal AWS endpoints therefore we override with a custom
	// endpoint resovler function.
	// If the host name is left as mutable, it will be changed to suit the normal host names for AWS
	// s3 buckets. This behavior would break the ability to connect with AME's object storage therfore
	// the HostNameImmatuable option is very important in the endpoint resolver function.
	staticResolver := aws.EndpointResolverFunc(func(service, region string) (aws.Endpoint, error) {
		return aws.Endpoint{
			URL:               endpoint,
			PartitionID:       "aws", // TODO: why aws here?
			HostnameImmutable: true,
			SigningRegion:     region, // TODO: what are the requirenents for the signing region?
		}, nil
	})

	// This grabs configuration from environment variables.
	cfg, err := config.LoadDefaultConfig(ctx)
	if err != nil {
		return nil, err
	}

	cfg.EndpointResolver = staticResolver
	s3Client := s3.NewFromConfig(cfg, overrider...)

	return s3Client, nil
}

func CreateS3ClientForLocalStorage(ctx context.Context) (*s3.Client, error) {
	return CreateS3Client(ctx, "http://127.0.0.1:9000", "", func(opts *s3.Options) {
		opts.EndpointOptions.DisableHTTPS = true
		opts.Credentials = credentials.NewStaticCredentialsProvider("minio", "minio123", "")
	})
}
