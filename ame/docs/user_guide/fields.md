# Protocol Documentation
<a name="top"></a>

## Table of Contents

- [lib/ame.proto](#lib_ame-proto)
    - [AmeSecret](#ame-v1-AmeSecret)
    - [AmeSecretId](#ame-v1-AmeSecretId)
    - [AmeSecretVariant](#ame-v1-AmeSecretVariant)
    - [AmeSecrets](#ame-v1-AmeSecrets)
    - [ArtifactCfg](#ame-v1-ArtifactCfg)
    - [CreateProjectRequest](#ame-v1-CreateProjectRequest)
    - [CustomExecutor](#ame-v1-CustomExecutor)
    - [DataSetCfg](#ame-v1-DataSetCfg)
    - [Empty](#ame-v1-Empty)
    - [EnvVar](#ame-v1-EnvVar)
    - [FileChunk](#ame-v1-FileChunk)
    - [GitProjectSource](#ame-v1-GitProjectSource)
    - [ListProjectSrcsResponse](#ame-v1-ListProjectSrcsResponse)
    - [ListTasksRequest](#ame-v1-ListTasksRequest)
    - [ListTasksResponse](#ame-v1-ListTasksResponse)
    - [ListTasksResponse.TasksEntry](#ame-v1-ListTasksResponse-TasksEntry)
    - [LogEntry](#ame-v1-LogEntry)
    - [MlflowExecutor](#ame-v1-MlflowExecutor)
    - [Model](#ame-v1-Model)
    - [ModelDeploymentCfg](#ame-v1-ModelDeploymentCfg)
    - [ModelDeploymentCfg.IngressAnnotationsEntry](#ame-v1-ModelDeploymentCfg-IngressAnnotationsEntry)
    - [ModelDeploymentCfg.ResourcesEntry](#ame-v1-ModelDeploymentCfg-ResourcesEntry)
    - [ModelStatus](#ame-v1-ModelStatus)
    - [ModelTrainingCfg](#ame-v1-ModelTrainingCfg)
    - [PipEnvExecutor](#ame-v1-PipEnvExecutor)
    - [PipExecutor](#ame-v1-PipExecutor)
    - [PoetryExecutor](#ame-v1-PoetryExecutor)
    - [ProjectCfg](#ame-v1-ProjectCfg)
    - [ProjectFileChunk](#ame-v1-ProjectFileChunk)
    - [ProjectFileIdentifier](#ame-v1-ProjectFileIdentifier)
    - [ProjectId](#ame-v1-ProjectId)
    - [ProjectSourceCfg](#ame-v1-ProjectSourceCfg)
    - [ProjectSourceId](#ame-v1-ProjectSourceId)
    - [ProjectSourceIssue](#ame-v1-ProjectSourceIssue)
    - [ProjectSourceListParams](#ame-v1-ProjectSourceListParams)
    - [ProjectSourceStatus](#ame-v1-ProjectSourceStatus)
    - [ProjectSrcIdRequest](#ame-v1-ProjectSrcIdRequest)
    - [ProjectSrcPatchRequest](#ame-v1-ProjectSrcPatchRequest)
    - [ProjectStatus](#ame-v1-ProjectStatus)
    - [ProjectStatus.ModelsEntry](#ame-v1-ProjectStatus-ModelsEntry)
    - [RemoveTaskRequest](#ame-v1-RemoveTaskRequest)
    - [ResourceCfg](#ame-v1-ResourceCfg)
    - [ResourceId](#ame-v1-ResourceId)
    - [ResourceIds](#ame-v1-ResourceIds)
    - [ResourceListParams](#ame-v1-ResourceListParams)
    - [RunTaskRequest](#ame-v1-RunTaskRequest)
    - [Secret](#ame-v1-Secret)
    - [TaskCfg](#ame-v1-TaskCfg)
    - [TaskCfg.ResourcesEntry](#ame-v1-TaskCfg-ResourcesEntry)
    - [TaskId](#ame-v1-TaskId)
    - [TaskIdentifier](#ame-v1-TaskIdentifier)
    - [TaskListEntry](#ame-v1-TaskListEntry)
    - [TaskLogRequest](#ame-v1-TaskLogRequest)
    - [TaskPhaseFailed](#ame-v1-TaskPhaseFailed)
    - [TaskPhasePending](#ame-v1-TaskPhasePending)
    - [TaskPhaseRunning](#ame-v1-TaskPhaseRunning)
    - [TaskPhaseSucceeded](#ame-v1-TaskPhaseSucceeded)
    - [TaskProjectDirectoryStructure](#ame-v1-TaskProjectDirectoryStructure)
    - [TaskRef](#ame-v1-TaskRef)
    - [TaskStatus](#ame-v1-TaskStatus)
    - [TemplateRef](#ame-v1-TemplateRef)
    - [TrainRequest](#ame-v1-TrainRequest)
    - [TriggerCfg](#ame-v1-TriggerCfg)
  
    - [ProjectSourceIssueType](#ame-v1-ProjectSourceIssueType)
    - [ProjectSourceState](#ame-v1-ProjectSourceState)
    - [TaskType](#ame-v1-TaskType)
  
    - [AmeService](#ame-v1-AmeService)
  
- [Scalar Value Types](#scalar-value-types)



<a name="lib_ame-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## lib/ame.proto



<a name="ame-v1-AmeSecret"></a>

### AmeSecret
This is a secret stored by AME


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="ame-v1-AmeSecretId"></a>

### AmeSecretId



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |






<a name="ame-v1-AmeSecretVariant"></a>

### AmeSecretVariant



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| injectAs | [string](#string) |  |  |






<a name="ame-v1-AmeSecrets"></a>

### AmeSecrets



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| secrets | [AmeSecretId](#ame-v1-AmeSecretId) | repeated |  |






<a name="ame-v1-ArtifactCfg"></a>

### ArtifactCfg



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| save_changed_files | [bool](#bool) |  |  |
| paths | [string](#string) | repeated |  |






<a name="ame-v1-CreateProjectRequest"></a>

### CreateProjectRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| cfg | [ProjectCfg](#ame-v1-ProjectCfg) |  |  |
| enableTriggers | [bool](#bool) | optional |  |






<a name="ame-v1-CustomExecutor"></a>

### CustomExecutor



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| pythonVersion | [string](#string) |  |  |
| command | [string](#string) |  |  |






<a name="ame-v1-DataSetCfg"></a>

### DataSetCfg



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  |  |
| task | [TaskCfg](#ame-v1-TaskCfg) |  |  |
| path | [string](#string) |  |  |
| size | [string](#string) | optional |  |






<a name="ame-v1-Empty"></a>

### Empty







<a name="ame-v1-EnvVar"></a>

### EnvVar



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| val | [string](#string) |  |  |






<a name="ame-v1-FileChunk"></a>

### FileChunk



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| contents | [bytes](#bytes) |  |  |






<a name="ame-v1-GitProjectSource"></a>

### GitProjectSource



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| repository | [string](#string) |  |  |
| username | [string](#string) | optional |  |
| secret | [string](#string) | optional |  |
| sync_interval | [string](#string) | optional |  |






<a name="ame-v1-ListProjectSrcsResponse"></a>

### ListProjectSrcsResponse



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| cfgs | [ProjectSourceCfg](#ame-v1-ProjectSourceCfg) | repeated |  |






<a name="ame-v1-ListTasksRequest"></a>

### ListTasksRequest







<a name="ame-v1-ListTasksResponse"></a>

### ListTasksResponse



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| tasks | [ListTasksResponse.TasksEntry](#ame-v1-ListTasksResponse-TasksEntry) | repeated |  |






<a name="ame-v1-ListTasksResponse-TasksEntry"></a>

### ListTasksResponse.TasksEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [TaskListEntry](#ame-v1-TaskListEntry) |  |  |






<a name="ame-v1-LogEntry"></a>

### LogEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| contents | [bytes](#bytes) |  |  |






<a name="ame-v1-MlflowExecutor"></a>

### MlflowExecutor







<a name="ame-v1-Model"></a>

### Model



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  |  |
| validationTask | [TaskCfg](#ame-v1-TaskCfg) | optional |  |
| training | [ModelTrainingCfg](#ame-v1-ModelTrainingCfg) | optional |  |
| deployment | [ModelDeploymentCfg](#ame-v1-ModelDeploymentCfg) | optional |  |






<a name="ame-v1-ModelDeploymentCfg"></a>

### ModelDeploymentCfg



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ingressAnnotations | [ModelDeploymentCfg.IngressAnnotationsEntry](#ame-v1-ModelDeploymentCfg-IngressAnnotationsEntry) | repeated |  |
| replicas | [int32](#int32) | optional |  |
| image | [string](#string) | optional |  |
| resources | [ModelDeploymentCfg.ResourcesEntry](#ame-v1-ModelDeploymentCfg-ResourcesEntry) | repeated |  |
| enableTls | [bool](#bool) | optional |  |






<a name="ame-v1-ModelDeploymentCfg-IngressAnnotationsEntry"></a>

### ModelDeploymentCfg.IngressAnnotationsEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="ame-v1-ModelDeploymentCfg-ResourcesEntry"></a>

### ModelDeploymentCfg.ResourcesEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="ame-v1-ModelStatus"></a>

### ModelStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| latestValidatedModelVersion | [string](#string) | optional |  |






<a name="ame-v1-ModelTrainingCfg"></a>

### ModelTrainingCfg



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task | [TaskCfg](#ame-v1-TaskCfg) |  |  |






<a name="ame-v1-PipEnvExecutor"></a>

### PipEnvExecutor



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| command | [string](#string) |  |  |






<a name="ame-v1-PipExecutor"></a>

### PipExecutor



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| pythonVersion | [string](#string) |  |  |
| command | [string](#string) |  |  |






<a name="ame-v1-PoetryExecutor"></a>

### PoetryExecutor



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| pythonVersion | [string](#string) |  |  |
| command | [string](#string) |  |  |






<a name="ame-v1-ProjectCfg"></a>

### ProjectCfg



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  |  |
| models | [Model](#ame-v1-Model) | repeated |  |
| dataSets | [DataSetCfg](#ame-v1-DataSetCfg) | repeated |  |
| tasks | [TaskCfg](#ame-v1-TaskCfg) | repeated |  |
| templates | [TaskCfg](#ame-v1-TaskCfg) | repeated |  |
| enableTriggers | [bool](#bool) | optional |  |






<a name="ame-v1-ProjectFileChunk"></a>

### ProjectFileChunk



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| chunk | [FileChunk](#ame-v1-FileChunk) |  |  |
| identifier | [ProjectFileIdentifier](#ame-v1-ProjectFileIdentifier) |  |  |






<a name="ame-v1-ProjectFileIdentifier"></a>

### ProjectFileIdentifier



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| taskid | [string](#string) |  |  |
| filepath | [string](#string) |  |  |






<a name="ame-v1-ProjectId"></a>

### ProjectId



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  |  |






<a name="ame-v1-ProjectSourceCfg"></a>

### ProjectSourceCfg



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| git | [GitProjectSource](#ame-v1-GitProjectSource) | optional |  |






<a name="ame-v1-ProjectSourceId"></a>

### ProjectSourceId



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  |  |






<a name="ame-v1-ProjectSourceIssue"></a>

### ProjectSourceIssue



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| issue_type | [ProjectSourceIssueType](#ame-v1-ProjectSourceIssueType) |  |  |
| explanation | [string](#string) | optional |  |






<a name="ame-v1-ProjectSourceListParams"></a>

### ProjectSourceListParams







<a name="ame-v1-ProjectSourceStatus"></a>

### ProjectSourceStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| last_synced | [string](#string) | optional |  |
| state | [ProjectSourceState](#ame-v1-ProjectSourceState) |  |  |
| reason | [string](#string) | optional |  |
| issues | [ProjectSourceIssue](#ame-v1-ProjectSourceIssue) | repeated |  |






<a name="ame-v1-ProjectSrcIdRequest"></a>

### ProjectSrcIdRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| repo | [string](#string) |  |  |






<a name="ame-v1-ProjectSrcPatchRequest"></a>

### ProjectSrcPatchRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id | [ProjectSourceId](#ame-v1-ProjectSourceId) |  |  |
| cfg | [ProjectSourceCfg](#ame-v1-ProjectSourceCfg) |  |  |






<a name="ame-v1-ProjectStatus"></a>

### ProjectStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| models | [ProjectStatus.ModelsEntry](#ame-v1-ProjectStatus-ModelsEntry) | repeated |  |






<a name="ame-v1-ProjectStatus-ModelsEntry"></a>

### ProjectStatus.ModelsEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [ModelStatus](#ame-v1-ModelStatus) |  |  |






<a name="ame-v1-RemoveTaskRequest"></a>

### RemoveTaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  |  |
| approve | [bool](#bool) | optional |  |






<a name="ame-v1-ResourceCfg"></a>

### ResourceCfg



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| projectSrcCfg | [ProjectSourceCfg](#ame-v1-ProjectSourceCfg) |  |  |






<a name="ame-v1-ResourceId"></a>

### ResourceId



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| projectSrcId | [ProjectSourceId](#ame-v1-ProjectSourceId) |  |  |






<a name="ame-v1-ResourceIds"></a>

### ResourceIds



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ids | [ResourceId](#ame-v1-ResourceId) | repeated |  |






<a name="ame-v1-ResourceListParams"></a>

### ResourceListParams



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| projectSourceListParams | [ProjectSourceListParams](#ame-v1-ProjectSourceListParams) |  |  |






<a name="ame-v1-RunTaskRequest"></a>

### RunTaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| projectId | [ProjectId](#ame-v1-ProjectId) |  |  |
| taskCfg | [TaskCfg](#ame-v1-TaskCfg) |  |  |






<a name="ame-v1-Secret"></a>

### Secret



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ame | [AmeSecretVariant](#ame-v1-AmeSecretVariant) |  |  |






<a name="ame-v1-TaskCfg"></a>

### TaskCfg



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) | optional |  |
| taskRef | [TaskRef](#ame-v1-TaskRef) | optional |  |
| resources | [TaskCfg.ResourcesEntry](#ame-v1-TaskCfg-ResourcesEntry) | repeated |  |
| poetry | [PoetryExecutor](#ame-v1-PoetryExecutor) |  |  |
| mlflow | [MlflowExecutor](#ame-v1-MlflowExecutor) |  |  |
| pipEnv | [PipEnvExecutor](#ame-v1-PipEnvExecutor) |  |  |
| pip | [PipExecutor](#ame-v1-PipExecutor) |  |  |
| custom | [CustomExecutor](#ame-v1-CustomExecutor) |  |  |
| dataSets | [string](#string) | repeated |  |
| fromTemplate | [TemplateRef](#ame-v1-TemplateRef) | optional |  |
| artifactCfg | [ArtifactCfg](#ame-v1-ArtifactCfg) | optional |  |
| triggers | [TriggerCfg](#ame-v1-TriggerCfg) | optional |  |
| env | [EnvVar](#ame-v1-EnvVar) | repeated |  |
| secrets | [Secret](#ame-v1-Secret) | repeated |  |






<a name="ame-v1-TaskCfg-ResourcesEntry"></a>

### TaskCfg.ResourcesEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="ame-v1-TaskId"></a>

### TaskId



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  |  |






<a name="ame-v1-TaskIdentifier"></a>

### TaskIdentifier



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  |  |






<a name="ame-v1-TaskListEntry"></a>

### TaskListEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| status | [TaskStatus](#ame-v1-TaskStatus) |  |  |
| timeStamp | [string](#string) |  |  |






<a name="ame-v1-TaskLogRequest"></a>

### TaskLogRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| taskid | [TaskIdentifier](#ame-v1-TaskIdentifier) |  |  |
| start_from | [int32](#int32) | optional |  |
| watch | [bool](#bool) | optional |  |






<a name="ame-v1-TaskPhaseFailed"></a>

### TaskPhaseFailed



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| workflowName | [string](#string) |  |  |






<a name="ame-v1-TaskPhasePending"></a>

### TaskPhasePending







<a name="ame-v1-TaskPhaseRunning"></a>

### TaskPhaseRunning



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| workflowName | [string](#string) |  |  |






<a name="ame-v1-TaskPhaseSucceeded"></a>

### TaskPhaseSucceeded



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| workflowName | [string](#string) |  |  |






<a name="ame-v1-TaskProjectDirectoryStructure"></a>

### TaskProjectDirectoryStructure



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| projectid | [string](#string) |  |  |
| taskid | [TaskIdentifier](#ame-v1-TaskIdentifier) |  |  |
| paths | [string](#string) | repeated |  |






<a name="ame-v1-TaskRef"></a>

### TaskRef



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  |  |
| project | [string](#string) | optional |  |






<a name="ame-v1-TaskStatus"></a>

### TaskStatus
TODO: should there be an error case?


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| pending | [TaskPhasePending](#ame-v1-TaskPhasePending) |  |  |
| running | [TaskPhaseRunning](#ame-v1-TaskPhaseRunning) |  |  |
| failed | [TaskPhaseFailed](#ame-v1-TaskPhaseFailed) |  |  |
| succeeded | [TaskPhaseSucceeded](#ame-v1-TaskPhaseSucceeded) |  |  |






<a name="ame-v1-TemplateRef"></a>

### TemplateRef



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  |  |
| project | [string](#string) | optional |  |






<a name="ame-v1-TrainRequest"></a>

### TrainRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| projectid | [string](#string) |  |  |
| model_name | [string](#string) |  |  |






<a name="ame-v1-TriggerCfg"></a>

### TriggerCfg



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| schedule | [string](#string) | optional |  |





 


<a name="ame-v1-ProjectSourceIssueType"></a>

### ProjectSourceIssueType


| Name | Number | Description |
| ---- | ------ | ----------- |
| Unknown | 0 |  |
| AuthFailure | 1 |  |
| RepositoryNotFound | 2 |  |
| AmeProjectNotFound | 3 |  |
| GitSecretNotFound | 4 |  |



<a name="ame-v1-ProjectSourceState"></a>

### ProjectSourceState


| Name | Number | Description |
| ---- | ------ | ----------- |
| Pending | 0 |  |
| Synchronising | 1 |  |
| Synchronized | 2 |  |
| Error | 3 |  |



<a name="ame-v1-TaskType"></a>

### TaskType


| Name | Number | Description |
| ---- | ------ | ----------- |
| Pipenv | 0 |  |
| Mlflow | 1 |  |
| Poetry | 2 |  |


 

 


<a name="ame-v1-AmeService"></a>

### AmeService


| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| RunTask | [RunTaskRequest](#ame-v1-RunTaskRequest) | [TaskIdentifier](#ame-v1-TaskIdentifier) |  |
| GetTask | [TaskIdentifier](#ame-v1-TaskIdentifier) | [TaskCfg](#ame-v1-TaskCfg) |  |
| DeleteTask | [TaskIdentifier](#ame-v1-TaskIdentifier) | [Empty](#ame-v1-Empty) |  |
| CreateTaskProjectDirectory | [TaskProjectDirectoryStructure](#ame-v1-TaskProjectDirectoryStructure) | [Empty](#ame-v1-Empty) |  |
| UploadProjectFile | [ProjectFileChunk](#ame-v1-ProjectFileChunk) stream | [Empty](#ame-v1-Empty) |  |
| StreamTaskLogs | [TaskLogRequest](#ame-v1-TaskLogRequest) | [LogEntry](#ame-v1-LogEntry) stream |  |
| CreateProjectSrc | [ProjectSourceCfg](#ame-v1-ProjectSourceCfg) | [ProjectSourceId](#ame-v1-ProjectSourceId) |  |
| DeleteProjectSrc | [ProjectSourceId](#ame-v1-ProjectSourceId) | [Empty](#ame-v1-Empty) |  |
| TrainModel | [TrainRequest](#ame-v1-TrainRequest) | [Empty](#ame-v1-Empty) |  |
| WatchProjectSrc | [ProjectSourceId](#ame-v1-ProjectSourceId) | [ProjectSourceStatus](#ame-v1-ProjectSourceStatus) stream |  |
| CreateSecret | [AmeSecret](#ame-v1-AmeSecret) | [Empty](#ame-v1-Empty) |  |
| DeleteSecret | [AmeSecretId](#ame-v1-AmeSecretId) | [Empty](#ame-v1-Empty) |  |
| ListSecrets | [Empty](#ame-v1-Empty) | [AmeSecrets](#ame-v1-AmeSecrets) |  |
| UpdateProjectSrc | [ProjectSrcPatchRequest](#ame-v1-ProjectSrcPatchRequest) | [Empty](#ame-v1-Empty) |  |
| CreateResource | [ResourceCfg](#ame-v1-ResourceCfg) | [ResourceId](#ame-v1-ResourceId) |  |
| ListResource | [ResourceListParams](#ame-v1-ResourceListParams) | [ResourceIds](#ame-v1-ResourceIds) |  |
| GetProjectSrcCfg | [ProjectSourceId](#ame-v1-ProjectSourceId) | [ProjectSourceCfg](#ame-v1-ProjectSourceCfg) |  |
| GetProjectSrcStatus | [ProjectSourceId](#ame-v1-ProjectSourceId) | [ProjectSourceStatus](#ame-v1-ProjectSourceStatus) |  |
| GetProjectSrcId | [ProjectSrcIdRequest](#ame-v1-ProjectSrcIdRequest) | [ProjectSourceId](#ame-v1-ProjectSourceId) |  |
| ListProjectSrcs | [ProjectSourceListParams](#ame-v1-ProjectSourceListParams) | [ListProjectSrcsResponse](#ame-v1-ListProjectSrcsResponse) |  |
| CreateProject | [CreateProjectRequest](#ame-v1-CreateProjectRequest) | [ProjectId](#ame-v1-ProjectId) |  |
| ListTasks | [ListTasksRequest](#ame-v1-ListTasksRequest) | [ListTasksResponse](#ame-v1-ListTasksResponse) |  |
| RemoveTask | [RemoveTaskRequest](#ame-v1-RemoveTaskRequest) | [Empty](#ame-v1-Empty) |  |

 



## Scalar Value Types

| .proto Type | Notes | C++ | Java | Python | Go | C# | PHP | Ruby |
| ----------- | ----- | --- | ---- | ------ | -- | -- | --- | ---- |
| <a name="double" /> double |  | double | double | float | float64 | double | float | Float |
| <a name="float" /> float |  | float | float | float | float32 | float | float | Float |
| <a name="int32" /> int32 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint32 instead. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="int64" /> int64 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint64 instead. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="uint32" /> uint32 | Uses variable-length encoding. | uint32 | int | int/long | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a name="uint64" /> uint64 | Uses variable-length encoding. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum or Fixnum (as required) |
| <a name="sint32" /> sint32 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int32s. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="sint64" /> sint64 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int64s. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="fixed32" /> fixed32 | Always four bytes. More efficient than uint32 if values are often greater than 2^28. | uint32 | int | int | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a name="fixed64" /> fixed64 | Always eight bytes. More efficient than uint64 if values are often greater than 2^56. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum |
| <a name="sfixed32" /> sfixed32 | Always four bytes. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="sfixed64" /> sfixed64 | Always eight bytes. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="bool" /> bool |  | bool | boolean | boolean | bool | bool | boolean | TrueClass/FalseClass |
| <a name="string" /> string | A string must always contain UTF-8 encoded or 7-bit ASCII text. | string | String | str/unicode | string | string | string | String (UTF-8) |
| <a name="bytes" /> bytes | May contain any arbitrary sequence of bytes. | string | ByteString | str | []byte | ByteString | string | String (ASCII-8BIT) |

