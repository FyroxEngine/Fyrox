#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <memory.h>
#include <stdlib.h>
#include <string.h>
#include <iostream>
#include <fstream>
#include <experimental/filesystem>

#define CONVHULL_3D_ENABLE
#include "convexhull_3d/convhull_3d.h"

using namespace std;
using namespace std::experimental::filesystem::v1;

typedef struct WavHeader {
	int8_t chunk_id[4];
	uint32_t chunk_size;
	int8_t format[4];
	/* fmt  chunk */ 
	int8_t fmt_chunk_id[4];
	uint32_t fmt_chunk_size;
	uint16_t audio_format;
	uint16_t num_channels;
	uint32_t sample_rate;
	uint32_t byte_rate;
	uint16_t block_align;
	uint16_t bits_per_sample;
	/* data chunk */
	int8_t data_chunk_id[4];
	uint32_t data_chunk_size;
} WavHeader;

template<typename T>
void ReadExact(ifstream& stream, T& v)
{
	const auto len = sizeof(T);
	stream.read(reinterpret_cast<char*>(&v), len);
	if (stream.eof()) {
		throw runtime_error("invalid wav");
	}
}

template<typename T>
void ReadExact(ifstream& stream, vector<T>& vec)
{
	const auto size = vec.size() * sizeof(T);
	stream.read(reinterpret_cast<char*>(vec.data()), size);
	if (stream.eof()) {
		throw runtime_error("invalid wav");
	}
}

template<typename T>
void WriteExact(ofstream& stream, const T& v)
{
	stream.write(reinterpret_cast<const char*>(&v), sizeof(T));
}

template<typename T>
void WriteExact(ofstream& stream, const vector<T>& v)
{
	stream.write(reinterpret_cast<const char*>(v.data()), v.size() * sizeof(T));
}

class SoundBuffer {
public:
	SoundBuffer(const string& file_name)
	{
		auto f = ifstream(file_name, ios::binary);

		if (!f.is_open()) {
			throw runtime_error("unable to open" + file_name);
		}

		WavHeader wav;

		ReadExact(f, wav.chunk_id);
		ReadExact(f, wav.chunk_size);
		ReadExact(f, wav.format);
		ReadExact(f, wav.fmt_chunk_id);
		ReadExact(f, wav.fmt_chunk_size);
		ReadExact(f, wav.audio_format);
		ReadExact(f, wav.num_channels);
		ReadExact(f, wav.sample_rate);
		ReadExact(f, wav.byte_rate);
		ReadExact(f, wav.block_align);
		ReadExact(f, wav.bits_per_sample);
		ReadExact(f, wav.data_chunk_id);
		ReadExact(f, wav.data_chunk_size);

		if (strncmp((char*)wav.chunk_id, "RIFF", sizeof(wav.chunk_id)) != 0) {
			throw runtime_error("wav: invalid chunk id");
		}

		if (strncmp((char*)wav.data_chunk_id, "data", sizeof(wav.data_chunk_id)) != 0) {
			throw runtime_error("invalid wav");
		}

		if (strncmp((char*)wav.fmt_chunk_id, "fmt ", sizeof(wav.fmt_chunk_id)) != 0) {
			throw runtime_error("wav: invalid fmt chunk id");
		}

		if (strncmp((char*)wav.format, "WAVE", sizeof(wav.format)) != 0) {
			throw runtime_error("wav: invalid format");
		}

		if (wav.audio_format != 1) {
			throw runtime_error("wav: compressed formats not supported!");
		}

		if (wav.num_channels != 2) {
			throw runtime_error("hrtf must have two channels!");
		}

		m_Data.resize(wav.data_chunk_size);
		ReadExact(f, m_Data);

		m_SampleSize = wav.bits_per_sample / 8;
		m_SampleRate = wav.sample_rate;
	}

	~SoundBuffer() = default;

	vector<char> m_Data;
	uint16_t m_SampleSize;
	uint32_t m_SampleRate;
};

struct Vec3 {
	float x;
	float y;
	float z;

	Vec3() :x(0), y(0), z(0)
	{
	}

	Vec3(float x, float y, float z) : x(x), y(y), z(z)
	{
	}
};

struct HrtfVertex {
	HrtfVertex(uint32_t sampleRate, const Vec3& position, vector<float>&& leftHRIR, vector<float>&& rightHRIR)
		: m_Position(position),
		m_SampleRate(sampleRate),
		m_LeftHRIR(std::move(leftHRIR)),
		m_RightHRIR(std::move(rightHRIR))
	{
	}

	uint32_t m_SampleRate;
	Vec3 m_Position;
	vector<float> m_LeftHRIR;
	vector<float> m_RightHRIR;
};

static char FileMagic[4] = { 'H', 'R', 'I', 'R' };

class HrtfSphere {
public:
	HrtfSphere() = default;
	~HrtfSphere() = default;

	void AddVertex(HrtfVertex&& v)
	{
		m_Vertices.push_back(std::move(v));
	}

	void Triangulate()
	{
		vector<ch_vertex> vertices;
		for (const auto& v : m_Vertices) {
			ch_vertex ch_v;
			ch_v.x = v.m_Position.x;
			ch_v.y = v.m_Position.y;
			ch_v.z = v.m_Position.z;
			vertices.push_back(ch_v);
		}

		int* outIndices = NULL;
		int faceCount = 0;
		convhull_3d_build(vertices.data(), static_cast<int>(vertices.size()), &outIndices, &faceCount);

		m_Indices.clear();
		for (int i = 0; i < faceCount * 3; ++i) {
			m_Indices.push_back(outIndices[i]);
		}

	//#ifdef VISUAL_DEBUG
		convhull_3d_export_obj(vertices.data(), static_cast<int>(vertices.size()), outIndices, faceCount, false, "test");
	//#endif
	}

	void Validate()
	{
		if (m_Vertices.empty()) {
			throw runtime_error("sphere is empty!");
		}

		const auto expectedHrirLen = m_Vertices[0].m_LeftHRIR.size();
		const auto expectedSampleRate = m_Vertices[0].m_SampleRate;
		for (const auto& v : m_Vertices) {
			if (v.m_LeftHRIR.size() != expectedHrirLen || v.m_RightHRIR.size() != expectedHrirLen) {
				throw runtime_error("HRIR length must be same across all files!");
			}
			if (v.m_SampleRate != expectedSampleRate) {
				throw runtime_error("HRIR must have same sample rate across all files!");
			}
		}
	}

	void Save(ofstream& file)
	{
		const uint32_t sampleRate = m_Vertices[0].m_SampleRate;
		const auto hrirLen = static_cast<uint32_t>(m_Vertices[0].m_LeftHRIR.size());
		const auto vertexCount = static_cast<uint32_t>(m_Vertices.size());
		const auto indexCount = static_cast<uint32_t>(m_Indices.size());

		// Header
		WriteExact(file, FileMagic);
		WriteExact(file, sampleRate);
		WriteExact(file, hrirLen);
		WriteExact(file, vertexCount);
		WriteExact(file, indexCount);

		// Index buffer
		WriteExact(file, m_Indices);

		// Vertices
		for (const auto& v : m_Vertices) {
			WriteExact(file, v.m_Position.x);
			WriteExact(file, v.m_Position.y);
			WriteExact(file, v.m_Position.z);

			WriteExact(file, v.m_LeftHRIR);
			WriteExact(file, v.m_RightHRIR);
		}

		file.flush();
	}

private:
	vector<HrtfVertex> m_Vertices;
	vector<uint32_t> m_Indices;
};

Vec3 SphericalToCartesian(float azimuth, float elevation, float radius)
{
	// Translates spherical to cartesian, where Y - up, Z - forward, X - right
	const float x = radius * sin(elevation) * sin(azimuth);
	const float y = radius * cos(elevation);
	const float z = -radius * sin(elevation) * cos(azimuth);
	return Vec3(x, y, z);
}

constexpr float Pi = 3.1415926535f;

float ToRadians(float degrees)
{
	return degrees / 180.0f * Pi;
}

Vec3 ParseFileName(const string& fileName)
{
	const auto azimuth_location = fileName.find("_T");
	if (azimuth_location == string::npos) {
		throw runtime_error("invalid file name");
	}
	const auto azimuth = static_cast<float>(atof(fileName.substr(azimuth_location + 2, 3).c_str()));

	const auto elevation_location = fileName.find("_P");
	if (elevation_location == string::npos) {
		throw runtime_error("invalid file name");
	}
	const auto elevation = 90.0f - static_cast<float>(atof(fileName.substr(elevation_location + 2, 3).c_str()));

	return SphericalToCartesian(ToRadians(azimuth), ToRadians(elevation), 1.0);
}

struct SamplePair {
	int m_Left;
	int m_Right;
};

SamplePair ReadSamplePair(const char* ptr, uint16_t sampleSize)
{
	SamplePair pair;
	if (sampleSize == 1) {
		pair.m_Left = *ptr;
		pair.m_Right = *(ptr + 1);
	} else if (sampleSize == 2) {
		const auto bit16 = reinterpret_cast<const short*>(ptr);
		pair.m_Left = *bit16;
		pair.m_Right = *(bit16 + 1);
	} else {
		throw runtime_error("sample size unsupported");
	}
	return pair;
}

int SampleLimit(uint16_t sampleSize)
{
	if (sampleSize == 1) {
		return std::numeric_limits<int8_t>::max();
	} else if (sampleSize == 2) {
		return std::numeric_limits<int16_t>::max();
	}
	throw runtime_error("sample size unsupported");
}

int main(int argc, char** arcv) try {
	if (argc != 2) {
		throw runtime_error("no path specified");
	}

	const char* folder = arcv[1];

	if (!is_directory(folder)) {
		throw runtime_error("path must be a folder!");
	}

	HrtfSphere sphere;

	for (const auto& entry : directory_iterator(folder)) {
		const auto path = entry.path().u8string();

		cout << "working on " << path << endl;

		const auto position = ParseFileName(path);
		const auto buffer = SoundBuffer(path);

		const auto sampleLimit = static_cast<float>(SampleLimit(buffer.m_SampleSize));

		vector<float> leftHRIR, rightHRIR;

		const char* ptr = buffer.m_Data.data();
		const char* end = ptr + buffer.m_Data.size();
		while (ptr != end) {
			const auto pair = ReadSamplePair(ptr, buffer.m_SampleSize);
			leftHRIR.push_back(static_cast<float>(pair.m_Left) / sampleLimit);
			rightHRIR.push_back(static_cast<float>(pair.m_Right) / sampleLimit);
			ptr += 2 * buffer.m_SampleSize;
		}

		sphere.AddVertex(HrtfVertex(buffer.m_SampleRate, position, std::move(leftHRIR), std::move(rightHRIR)));
	}

	sphere.Validate();
	sphere.Triangulate();

	ofstream output("hrir_base.bin", ios::binary);
	sphere.Save(output);

	cout << "done. saved into hrir_base.bin" << endl;

} catch (exception& e) {
	cerr << e.what();
}