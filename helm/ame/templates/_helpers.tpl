{{/* 
Create ame server name and version from the chart label 
*/}}
{{- define "ame.server.fullname" -}}
{{- printf "%s-%s" (include "ame.fullname" .) .Values.server.name | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/* Create ame controller name and version from the chart label */}}
{{- define "ame.controller.fullname" -}}
{{- printf "%s-%s" (include "ame.fullname" .) .Values.controller.name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Expand the name of the chart.
*/}}
{{- define "ame.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "ame.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}


{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "ame.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "ame.labels" -}}
helm.sh/chart: {{ include "ame.chart" .context }}
{{ include "ame.selectorLabels" (dict "context" .context "component" .component "name" .name) }}
app.kubernetes.io/part-of: ame
{{- end }}

{{/*
Selector labels
*/}}
{{- define "ame.selectorLabels" -}}
{{- if .name -}}
app.kubernetes.io/name: {{ include "ame.name" .context }}-{{ .name }}
{{ end -}}
{{- if .component }}
app.kubernetes.io/component: {{ .component }}
{{- end }}
{{- end }}