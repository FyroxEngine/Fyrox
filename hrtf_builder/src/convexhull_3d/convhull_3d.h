/*
 Copyright (c) 2017-2018 Leo McCormack
 
 Permission is hereby granted, free of charge, to any person obtaining a copy
 of this software and associated documentation files (the "Software"), to deal
 in the Software without restriction, including without limitation the rights
 to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 copies of the Software, and to permit persons to whom the Software is
 furnished to do so, subject to the following conditions:
 
 The above copyright notice and this permission notice shall be included in
 all copies or substantial portions of the Software.
 
 THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 THE SOFTWARE.
*/
/*
 * Filename:
 *     convhull_3d.h
 * Description:
 *     A header only C implementation of the 3-D quickhull algorithm.
 *     The code is largely derived from the "computational-geometry-toolbox"
 *     by George Papazafeiropoulos (c) 2014, originally distributed under
 *     the BSD (2-clause) license.
 *     To include this implementation in a project, simply add this:
 *         #define CONVHULL_3D_ENABLE
 *         #include "convhull_3d.h"
 *     By default, the algorithm uses double floating point precision. To
 *     use single precision (less accurate but quicker), also add this:
 *         #define CONVHULL_3D_USE_FLOAT_PRECISION
 *     If your project has CBLAS linked, then you can also speed things up
 *     a tad by adding this:
 *         #define CONVHULL_3D_USE_CBLAS
 *     The code is C++ compiler safe.
 *     Reference: "The Quickhull Algorithm for Convex Hull, C. Bradford
 *                 Barber, David P. Dobkin and Hannu Huhdanpaa, Geometry
 *                 Center Technical Report GCG53, July 30, 1993"
 * Dependencies:
 *     cblas (optional for speed ups, especially for very large meshes)
 * Author, date created:
 *     Leo McCormack, 02.10.2017
 *     
 * Modified: 
 *	Dmitry Stepanov 2019 - fixed compiler warnings.
 */

/**********
 * PUBLIC:
 *********/

#ifndef CONVHULL_3D_INCLUDED
#define CONVHULL_3D_INCLUDED

#ifdef __cplusplus
extern "C" {
#endif

    
#ifdef CONVHULL_3D_USE_FLOAT_PRECISION
typedef float CH_FLOAT;
#else
typedef double CH_FLOAT;
#endif
typedef struct _ch_vertex {
    union {
        CH_FLOAT v[3];
        struct{
             CH_FLOAT x, y, z;
        };
    };
} ch_vertex;
typedef ch_vertex ch_vec3;
   
/* builds the convexhull, returning the face indices corresponding to "in_vertices" */
void convhull_3d_build(/* input arguments */
                       ch_vertex* const in_vertices,            /* vector of input vertices; nVert x 1 */
                       const int nVert,                         /* number of vertices */
                       /* output arguments */
                       int** out_faces,                         /* & of empty int*, output face indices; flat: nOut_faces x 3 */
                       int* nOut_faces);                        /* & of int, number of output face indices */
    
/* exports the vertices, face indices, and face normals, as an 'obj' file, ready for GPU */
void convhull_3d_export_obj(/* input arguments */
                            ch_vertex* const vertices,          /* vector of input vertices; nVert x 1 */
                            const int nVert,                    /* number of vertices */
                            int* const faces,                   /* face indices; flat: nFaces x 3 */
                            const int nFaces,                   /* number of faces in hull */
                            const int keepOnlyUsedVerticesFLAG, /* 0: exports in_vertices, 1: exports only used vertices  */
                            char* const obj_filename);          /* obj filename, WITHOUT extension */
    
/* exports the vertices, face indices, and face normals, as an 'm' file, for MatLab verification */
void convhull_3d_export_m(/* input arguments */
                          ch_vertex* const vertices,            /* vector of input vertices; nVert x 1 */
                          const int nVert,                      /* number of vertices */
                          int* const faces,                     /* face indices; flat: nFaces x 3 */
                          const int nFaces,                     /* number of faces in hull */
                          char* const m_filename);              /* m filename, WITHOUT extension */
    
/* reads an 'obj' file and extracts only the vertices */
void extractVerticesFromObjFile(/* input arguments */
                                char* const obj_filename,       /* obj filename, WITHOUT extension */
                                /* output arguments */
                                ch_vertex** out_vertices,       /* & of empty ch_vertex*, output vertices; out_nVert x 1 */
                                int* out_nVert);                /* & of int, number of vertices */

#ifdef __cplusplus
} /*extern "C"*/
#endif

#endif /* CONVHULL_3D_INCLUDED */


/************
 * INTERNAL:
 ***********/

#ifdef CONVHULL_3D_ENABLE

#include <stdlib.h>
#include <stdio.h>
#include <math.h>
#include <string.h>
#include <float.h>
#include <ctype.h>
#include <string.h>
#if defined(_MSC_VER) && !defined(_CRT_SECURE_NO_WARNINGS)
  #define CV_STRNCPY(a,b,c) strncpy_s(a,c+1,b,c);
  #define CV_STRCAT(a,b) strcat_s(a,sizeof(b),b);
#else
  #define CV_STRNCPY(a,b,c) strncpy(a,b,c);
  #define CV_STRCAT(a,b) strcat(a,b);
#endif
#ifdef CONVHULL_3D_USE_FLOAT_PRECISION
  #define CH_FLT_MIN FLT_MIN
  #define CH_FLT_MAX FLT_MAX
  #define CH_NOISE_VAL 0.00001f
#else
  #define CH_FLT_MIN DBL_MIN
  #define CH_FLT_MAX DBL_MAX
  #define CH_NOISE_VAL 0.0000001
#endif
#ifndef MIN
  #define MIN(a,b) (( (a) < (b) ) ? (a) : (b) )
#endif
#ifndef MAX
  #define MAX(a,b) (( (a) > (b) ) ? (a) : (b) )
#endif
#define CH_MAX_NUM_FACES 50000

/* structs for qsort */
typedef struct float_w_idx {
    CH_FLOAT val;
    int idx;
}float_w_idx;

typedef struct int_w_idx {
    int val;
    int idx;
}int_w_idx;

/* internal functions prototypes: */
static int cmp_asc_float(const void*, const void*);
static int cmp_desc_float(const void*, const void*);
static int cmp_asc_int(const void*, const void*);
static int cmp_desc_int(const void*, const void*);
static void sort_float(CH_FLOAT*, CH_FLOAT*, int*, int, int);
static void sort_int(int*, int*, int*, int, int);
static ch_vec3 cross(ch_vec3*, ch_vec3*);
static CH_FLOAT det_4x4(CH_FLOAT*);
static void plane_3d(CH_FLOAT*, CH_FLOAT*, CH_FLOAT*);
static void ismember(int*, int*, int*, int, int);

/* internal functions definitions: */
static int cmp_asc_float(const void *a,const void *b) {
    struct float_w_idx *a1 = (struct float_w_idx*)a;
    struct float_w_idx *a2 = (struct float_w_idx*)b;
    if((*a1).val<(*a2).val)return -1;
    else if((*a1).val>(*a2).val)return 1;
    else return 0;
}

static int cmp_desc_float(const void *a,const void *b) {
    struct float_w_idx *a1 = (struct float_w_idx*)a;
    struct float_w_idx *a2 = (struct float_w_idx*)b;
    if((*a1).val>(*a2).val)return -1;
    else if((*a1).val<(*a2).val)return 1;
    else return 0;
}

static int cmp_asc_int(const void *a,const void *b) {
    struct int_w_idx *a1 = (struct int_w_idx*)a;
    struct int_w_idx *a2 = (struct int_w_idx*)b;
    if((*a1).val<(*a2).val)return -1;
    else if((*a1).val>(*a2).val)return 1;
    else return 0;
}

static int cmp_desc_int(const void *a,const void *b) {
    struct int_w_idx *a1 = (struct int_w_idx*)a;
    struct int_w_idx *a2 = (struct int_w_idx*)b;
    if((*a1).val>(*a2).val)return -1;
    else if((*a1).val<(*a2).val)return 1;
    else return 0;
}

static void sort_float
(
    CH_FLOAT* in_vec,  /* vector[len] to be sorted */
    CH_FLOAT* out_vec, /* if NULL, then in_vec is sorted "in-place" */
    int* new_idices,   /* set to NULL if you don't need them */
    int len,           /* number of elements in vectors, must be consistent with the input data */
    int descendFLAG    /* !1:ascending, 1:descending */
)
{
    int i;
    struct float_w_idx *data;
    
    data = (float_w_idx*)malloc(len*sizeof(float_w_idx));
    for(i=0;i<len;i++) {
        data[i].val=in_vec[i];
        data[i].idx=i;
    }
    if(descendFLAG)
        qsort(data,len,sizeof(data[0]),cmp_desc_float);
    else
        qsort(data,len,sizeof(data[0]),cmp_asc_float);
    for(i=0;i<len;i++){
        if (out_vec!=NULL)
            out_vec[i] = data[i].val;
        else
            in_vec[i] = data[i].val; /* overwrite input vector */
        if(new_idices!=NULL)
            new_idices[i] = data[i].idx;
    }
    free(data);
}

static void sort_int
(
    int* in_vec,     /* vector[len] to be sorted */
    int* out_vec,    /* if NULL, then in_vec is sorted "in-place" */
    int* new_idices, /* set to NULL if you don't need them */
    int len,         /* number of elements in vectors, must be consistent with the input data */
    int descendFLAG  /* !1:ascending, 1:descending */
)
{
    int i;
    struct int_w_idx *data;
    
    data = (int_w_idx*)malloc(len*sizeof(int_w_idx));
    for(i=0;i<len;i++) {
        data[i].val=in_vec[i];
        data[i].idx=i;
    }
    if(descendFLAG)
        qsort(data,len,sizeof(data[0]),cmp_desc_int);
    else
        qsort(data,len,sizeof(data[0]),cmp_asc_int);
    for(i=0;i<len;i++){
        if (out_vec!=NULL)
            out_vec[i] = data[i].val;
        else
            in_vec[i] = data[i].val; /* overwrite input vector */
        if(new_idices!=NULL)
            new_idices[i] = data[i].idx;
    }
    free(data);
}

static ch_vec3 cross(ch_vec3* v1, ch_vec3* v2)
{
    ch_vec3 cross;
    cross.x = v1->y * v2->z - v1->z * v2->y;
    cross.y = v1->z * v2->x - v1->x * v2->z;
    cross.z = v1->x * v2->y - v1->y * v2->x;
    return cross;
}

/* calculates the determinent of a 4x4 matrix */
static CH_FLOAT det_4x4(CH_FLOAT* m) {
    return
    m[3] * m[6] * m[9] * m[12] - m[2] * m[7] * m[9] * m[12] -
    m[3] * m[5] * m[10] * m[12] + m[1] * m[7] * m[10] * m[12] +
    m[2] * m[5] * m[11] * m[12] - m[1] * m[6] * m[11] * m[12] -
    m[3] * m[6] * m[8] * m[13] + m[2] * m[7] * m[8] * m[13] +
    m[3] * m[4] * m[10] * m[13] - m[0] * m[7] * m[10] * m[13] -
    m[2] * m[4] * m[11] * m[13] + m[0] * m[6] * m[11] * m[13] +
    m[3] * m[5] * m[8] * m[14] - m[1] * m[7] * m[8] * m[14] -
    m[3] * m[4] * m[9] * m[14] + m[0] * m[7] * m[9] * m[14] +
    m[1] * m[4] * m[11] * m[14] - m[0] * m[5] * m[11] * m[14] -
    m[2] * m[5] * m[8] * m[15] + m[1] * m[6] * m[8] * m[15] +
    m[2] * m[4] * m[9] * m[15] - m[0] * m[6] * m[9] * m[15] -
    m[1] * m[4] * m[10] * m[15] + m[0] * m[5] * m[10] * m[15];
}

/* Calculates the coefficients of the equation of a PLANE in 3D.
 * Original Copyright (c) 2014, George Papazafeiropoulos
 * Distributed under the BSD (2-clause) license
 */
static void plane_3d
(
    CH_FLOAT* p,
    CH_FLOAT* c,
    CH_FLOAT* d
)
{
    int i, j, k, l;
    int r[3];
    CH_FLOAT sign, det, norm_c;
    CH_FLOAT pdiff[2][3], pdiff_s[2][2];
    
    for(i=0; i<2; i++)
        for(j=0; j<3; j++)
            pdiff[i][j] = p[(i+1)*3+j] - p[i*3+j];
    memset(c, 0, 3*sizeof(CH_FLOAT));
    sign = 1.0;
    for(i=0; i<3; i++)
        r[i] = i;
    for(i=0; i<3; i++){
        for(j=0; j<2; j++){
            for(k=0, l=0; k<3; k++){
                if(r[k]!=i){
                    pdiff_s[j][l] = pdiff[j][k];
                    l++;
                }
            }
        }
        det = pdiff_s[0][0]*pdiff_s[1][1] - pdiff_s[1][0]*pdiff_s[0][1];
        c[i] = sign * det;
        sign *= -1.0;
    }
    norm_c = (CH_FLOAT)0.0;
    for(i=0; i<3; i++)
        norm_c += (pow(c[i], 2.0));
    norm_c = sqrt(norm_c);
    for(i=0; i<3; i++)
        c[i] /= norm_c;
    (*d) = (CH_FLOAT)0.0;
    for(i=0; i<3; i++)
        (*d) += -p[i] * c[i];
}

static void ismember
(
    int* pLeft,          /* left vector; nLeftElements x 1 */
    int* pRight,         /* right vector; nRightElements x 1 */
    int* pOut,           /* 0, unless pRight elements are present in pLeft then 1; nLeftElements x 1 */
    int nLeftElements,   /* number of elements in pLeft */
    int nRightElements   /* number of elements in pRight */
)
{
    int i, j;
    memset(pOut, 0, nLeftElements*sizeof(int));
    for(i=0; i< nLeftElements; i++)
        for(j=0; j< nRightElements; j++)
            if(pLeft[i] == pRight[j] )
                pOut[i] = 1;
}

/* A C version of the 3D quickhull matlab implementation from here:
 * https://www.mathworks.com/matlabcentral/fileexchange/48509-computational-geometry-toolbox?focused=3851550&tab=example
 * (*out_faces) is returned as NULL, if triangulation fails *
 * Original Copyright (c) 2014, George Papazafeiropoulos
 * Distributed under the BSD (2-clause) license
 * Reference: "The Quickhull Algorithm for Convex Hull, C. Bradford Barber, David P. Dobkin
 *             and Hannu Huhdanpaa, Geometry Center Technical Report GCG53, July 30, 1993"
 */
void convhull_3d_build
(
    ch_vertex* const in_vertices,
    const int nVert,
    int** out_faces,
    int* nOut_faces
)
{
    int i, j, k, l, h;
    int nFaces, p, d;
    int* aVec, *faces;
    CH_FLOAT dfi, v, max_p, min_p;
    CH_FLOAT* points, *cf, *cfi, *df, *p_s, *span;
    
    if(nVert<3 || in_vertices==NULL){
        (*out_faces) = NULL;
        (*nOut_faces) = 0;
        return;
    }
    
    /* 3 dimensions. The code should theoretically work for >=2 dimensions, but "plane_3d" and "det_4x4" are hardcoded for 3,
     * so would need to be rewritten */
    d = 3;
    span = (CH_FLOAT*)malloc(d*sizeof(CH_FLOAT));
    for(j=0; j<d; j++){
        max_p = 2.23e-13; min_p = 2.23e+13;
        for(i=0; i<nVert; i++){
            max_p = MAX(max_p, in_vertices[i].v[j]);
            min_p = MIN(min_p, in_vertices[i].v[j]);
        }
        span[j] = max_p - min_p;
    }
    points = (CH_FLOAT*)malloc(nVert*(d+1)*sizeof(CH_FLOAT));
    for(i=0; i<nVert; i++){
        for(j=0; j<d; j++)
            points[i*(d+1)+j] = in_vertices[i].v[j] + CH_NOISE_VAL*rand()/(float)RAND_MAX; /* noise mitigates duplicates */
        points[i*(d+1)+d] = 1.0f; /* add a last column of ones. Used only for determinant calculation */
    }
    
    /* The initial convex hull is a simplex with (d+1) facets, where d is the number of dimensions */
    nFaces = (d+1);
    faces = (int*)calloc(nFaces*d, sizeof(int));
    aVec = (int*)malloc(nFaces*sizeof(int));
    for(i=0; i<nFaces; i++)
        aVec[i] = i;
    
    /* Each column of cf contains the coefficients of a plane */
    cf = (CH_FLOAT*)malloc(nFaces*d*sizeof(CH_FLOAT));
    cfi = (CH_FLOAT*)malloc(d*sizeof(CH_FLOAT));
    df = (CH_FLOAT*)malloc(nFaces*sizeof(CH_FLOAT));
    p_s = (CH_FLOAT*)malloc(d*d*sizeof(CH_FLOAT));
    for(i=0; i<nFaces; i++){
        /* Set the indices of the points defining the face  */
        for(j=0, k=0; j<(d+1); j++){
            if(aVec[j]!=i){
                faces[i*d+k] = aVec[j];
                k++;
            }
        }
        
        /* Calculate and store the plane coefficients of the face */
        for(j=0; j<d; j++)
            for(k=0; k<d; k++)
                p_s[j*d+k] = points[(faces[i*d+j])*(d+1) + k];
        
        /* Calculate and store the plane coefficients of the face */
        plane_3d(p_s, cfi, &dfi);
        for(j=0; j<d; j++)
            cf[i*d+j] = cfi[j];
        df[i] = dfi;
    }
    CH_FLOAT *A;
    int *bVec, *fVec, *asfVec, *face_tmp;
    
    /* Check to make sure that faces are correctly oriented */
    bVec = (int*)malloc(4*sizeof(int));
    for(i=0; i<d+1; i++)
        bVec[i] = i;
    
    /* A contains the coordinates of the points forming a simplex */
    A = (CH_FLOAT*)calloc((d+1)*(d+1), sizeof(CH_FLOAT));
    face_tmp = (int*)malloc((d+1)*sizeof(int));
    fVec = (int*)malloc((d+1)*sizeof(int));
    asfVec = (int*)malloc((d+1)*sizeof(int));
    for(k=0; k<(d+1); k++){
        /* Get the point that is not on the current face (point p) */
        for(i=0; i<d; i++)
            fVec[i] = faces[k*d+i];
        sort_int(fVec, NULL, NULL, d, 0); /* sort accending */
        p=k;
        for(i=0; i<d; i++)
            for(j=0; j<(d+1); j++)
                A[i*(d+1)+j] = points[(faces[k*d+i])*(d+1) + j];
        for(; i<(d+1); i++)
            for(j=0; j<(d+1); j++)
                A[i*(d+1)+j] = points[p*(d+1)+j];
        
        /* det(A) determines the orientation of the face */
        v = det_4x4(A);
        
        /* Orient so that each point on the original simplex can't see the opposite face */
        if(v<0){
            /* Reverse the order of the last two vertices to change the volume */
            for(j=0; j<d; j++)
                face_tmp[j] = faces[k*d+j];
            for(j=0, l=d-2; j<d-1; j++, l++)
                faces[k*d+l] = face_tmp[d-j-1];
            
            /* Modify the plane coefficients of the properly oriented faces */
            for(j=0; j<d; j++)
                cf[k*d+j] = -cf[k*d+j];
            df[k] = -df[k];
            for(i=0; i<d; i++)
                for(j=0; j<(d+1); j++)
                    A[i*(d+1)+j] = points[(faces[k*d+i])*(d+1) + j];
            for(; i<(d+1); i++)
                for(j=0; j<(d+1); j++)
                    A[i*(d+1)+j] = points[p*(d+1)+j];
        }
    }
    
    /* Coordinates of the center of the point set */
    CH_FLOAT* meanp, *absdist, *reldist, *desReldist;
    meanp = (CH_FLOAT*)calloc(d, sizeof(CH_FLOAT));
    for(i=d+1; i<nVert; i++)
        for(j=0; j<d; j++)
            meanp[j] += points[i*(d+1)+j];
    for(j=0; j<d; j++)
        meanp[j] = meanp[j]/(CH_FLOAT)(nVert-d-1);
    
    /* Absolute distance of points from the center */
    absdist = (CH_FLOAT*)malloc((nVert-d-1)*d * sizeof(CH_FLOAT));
    for(i=d+1, k=0; i<nVert; i++, k++)
        for(j=0; j<d; j++)
            absdist[k*d+j] = (points[i*(d+1)+j] -  meanp[j])/span[j];
    
    /* Relative distance of points from the center */
    reldist = (CH_FLOAT*)calloc((nVert-d-1), sizeof(CH_FLOAT));
    desReldist = (CH_FLOAT*)malloc((nVert-d-1) * sizeof(CH_FLOAT));
    for(i=0; i<(nVert-d-1); i++)
        for(j=0; j<d; j++)
            reldist[i] += pow(absdist[i*d+j], 2.0);
    
    /* Sort from maximum to minimum relative distance */
    int num_pleft, cnt;
    int* ind, *pleft;
    ind = (int*)malloc((nVert-d-1) * sizeof(int));
    pleft = (int*)malloc((nVert-d-1) * sizeof(int));
    sort_float(reldist, desReldist, ind, (nVert-d-1), 1);
    
    /* Initialize the vector of points left. The points with the larger relative
     distance from the center are scanned first. */
    num_pleft = (nVert-d-1);
    for(i=0; i<num_pleft; i++)
        pleft[i] = ind[i]+d+1;
    
    /* Loop over all remaining points that are not deleted. Deletion of points
     occurs every #iter2del# iterations of this while loop */
    memset(A, 0, (d+1)*(d+1) * sizeof(CH_FLOAT));
    
    /* cnt is equal to the points having been selected without deletion of
     nonvisible points (i.e. points inside the current convex hull) */
    cnt=0;
    
    /* The main loop for the quickhull algorithm */
    CH_FLOAT detA;
    CH_FLOAT* points_cf, *points_s;
    int* visible_ind, *visible, *nonvisible_faces, *f0, *face_s, *u, *gVec, *horizon, *hVec, *pp, *hVec_mem_face;
    int num_visible_ind, num_nonvisible_faces, n_newfaces, count, vis;
    int f0_sum, u_len, start, num_p, index, horizon_size1;
    int FUCKED;
    FUCKED = 0;
    u = horizon = NULL;
    nFaces = d+1;
    visible_ind = (int*)malloc(nFaces*sizeof(int));
    points_cf = (CH_FLOAT*)malloc(nFaces*sizeof(CH_FLOAT));
    points_s = (CH_FLOAT*)malloc(d*sizeof(CH_FLOAT));
    face_s = (int*)malloc(d*sizeof(int));
    gVec = (int*)malloc(d*sizeof(int));
    while( (num_pleft>0) ){
        /* i is the first point of the points left */
        i = pleft[0];
        
        /* Delete the point selected */
        for(j=0; j<num_pleft-1; j++)
            pleft[j] = pleft[j+1];
        num_pleft--;
        if(num_pleft == 0)
            free(pleft);
        else
            pleft = (int*)realloc(pleft, num_pleft*sizeof(int));
        
        /* Update point selection counter */
        cnt++;
        
        /* find visible faces */
        for(j=0; j<d; j++)
            points_s[j] = points[i*(d+1)+j];
        points_cf = (CH_FLOAT*)realloc(points_cf, nFaces*sizeof(CH_FLOAT));
        visible_ind = (int*)realloc(visible_ind, nFaces*sizeof(int));
#ifdef CONVHULL_3D_USE_CBLAS
  #ifdef CONVHULL_3D_USE_FLOAT_PRECISION
        cblas_sgemm(CblasRowMajor, CblasNoTrans, CblasTrans, 1, nFaces, d, 1.0f,
                    points_s, d,
                    cf, d, 0.0f,
                    points_cf, nFaces);
  #else
        cblas_dgemm(CblasRowMajor, CblasNoTrans, CblasTrans, 1, nFaces, d, 1.0,
                    points_s, d,
                    cf, d, 0.0,
                    points_cf, nFaces);
  #endif
#else
        for (j = 0; j < nFaces; j++) {
            points_cf[j] = 0;
            for (k = 0; k < d; k++)
                points_cf[j] += points_s[k]*cf[j*d+k];
        }
#endif
        num_visible_ind = 0;
        for(j=0; j<nFaces; j++){
            if(points_cf[j] + df[j] > 0.0){
                num_visible_ind++; /* will sum to 0 if none are visible */
                visible_ind[j] = 1;
            }
            else
                visible_ind[j] = 0;
        }
        num_nonvisible_faces = nFaces - num_visible_ind;
        
        /* proceed if there are any visible faces */
        if(num_visible_ind!=0){
            /* Find visible face indices */
            visible = (int*)malloc(num_visible_ind*sizeof(int));
            for(j=0, k=0; j<nFaces; j++){
                if(visible_ind[j]==1){
                    visible[k]=j;
                    k++;
                }
            }
            
            /* Find nonvisible faces */
            nonvisible_faces = (int*)malloc(num_nonvisible_faces*d*sizeof(int));
            f0 = (int*)malloc(num_nonvisible_faces*d*sizeof(int));
            for(j=0, k=0; j<nFaces; j++){
                if(visible_ind[j]==0){
                    for(l=0; l<d; l++)
                        nonvisible_faces[k*d+l]= faces[j*d+l];
                    k++;
                }
            }
            
            /* Create horizon (count is the number of the edges of the horizon) */
            count=0;
            for(j=0; j<num_visible_ind; j++){
                /* visible face */
                vis = visible[j];
                for(k=0; k<d; k++)
                    face_s[k] = faces[vis*d+k];
                sort_int(face_s, NULL, NULL, d, 0);
                ismember(nonvisible_faces, face_s, f0, num_nonvisible_faces*d, d);
                u_len = 0;
                
                /* u are the nonvisible faces connected to the face v, if any */
                for(k=0; k<num_nonvisible_faces; k++){
                    f0_sum = 0;
                    for(l=0; l<d; l++)
                        f0_sum += f0[k*d + l];
                    if(f0_sum == d-1){
                        u_len++;
                        if(u_len==1)
                            u = (int*)malloc(u_len*sizeof(int));
                        else
                            u = (int*)realloc(u, u_len*sizeof(int));
                        u[u_len-1] = k;
                    }
                }
                for(k=0; k<u_len; k++){
                    /* The boundary between the visible face v and the k(th) nonvisible face connected to the face v forms part of the horizon */
                    count++;
                    if(count==1)
                        horizon = (int*)malloc(count*(d-1)*sizeof(int));
                    else
                        horizon = (int*)realloc(horizon, count*(d-1)*sizeof(int));
                    for(l=0; l<d; l++)
                        gVec[l] = nonvisible_faces[u[k]*d+l];
                    for(l=0, h=0; l<d; l++){
                        if(f0[u[k]*d+l]){
                            horizon[(count-1)*(d-1)+h] = gVec[l];
                            h++;
                        }
                    }
                }
                if(u_len!=0)
                    free(u);
            }
            horizon_size1 = count;
            for(j=0, l=0; j<nFaces; j++){
                if(!visible_ind[j]){
                    /* Delete visible faces */
                    for(k=0; k<d; k++)
                        faces[l*d+k] = faces[j*d+k];
                    
                    /* Delete the corresponding plane coefficients of the faces */
                    for(k=0; k<d; k++)
                        cf[l*d+k] = cf[j*d+k];
                    df[l] = df[j];
                    l++;
                }
            }
            
            /* Update the number of faces */
            nFaces = nFaces-num_visible_ind;
            faces = (int*)realloc(faces, nFaces*d*sizeof(int));
            cf = (CH_FLOAT*)realloc(cf, nFaces*d*sizeof(CH_FLOAT));
            df = (CH_FLOAT*)realloc(df, nFaces*sizeof(CH_FLOAT));
            
            /* start is the first row of the new faces */
            start=nFaces;
            
            /* Add faces connecting horizon to the new point */
            n_newfaces = horizon_size1;
            for(j=0; j<n_newfaces; j++){
                nFaces++;
                faces = (int*)realloc(faces, nFaces*d*sizeof(int));
                cf = (CH_FLOAT*)realloc(cf, nFaces*d*sizeof(CH_FLOAT));
                df = (CH_FLOAT*)realloc(df, nFaces*sizeof(CH_FLOAT));
                for(k=0; k<d-1; k++)
                    faces[(nFaces-1)*d+k] = horizon[j*(d-1)+k];
                faces[(nFaces-1)*d+(d-1)] = i;
                
                /* Calculate and store appropriately the plane coefficients of the faces */
                for(k=0; k<d; k++)
                    for(l=0; l<d; l++)
                        p_s[k*d+l] = points[(faces[(nFaces-1)*d+k])*(d+1) + l];
                plane_3d(p_s, cfi, &dfi);
                for(k=0; k<d; k++)
                    cf[(nFaces-1)*d+k] = cfi[k];
                df[(nFaces-1)] = dfi;
                if(nFaces > CH_MAX_NUM_FACES){
                    FUCKED = 1;
                    nFaces = 0;
                    break;
                }
            }
            
            /* Orient each new face properly */
            hVec = (int*)malloc( nFaces*sizeof(int));
            hVec_mem_face = (int*)malloc( nFaces*sizeof(int));
            for(j=0; j<nFaces; j++)
                hVec[j] = j;
            for(k=start; k<nFaces; k++){
                for(j=0; j<d; j++)
                    face_s[j] = faces[k*d+j];
                sort_int(face_s, NULL, NULL, d, 0);
                ismember(hVec, face_s, hVec_mem_face, nFaces, d);
                num_p = 0;
                for(j=0; j<nFaces; j++)
                    if(!hVec_mem_face[j])
                        num_p++;
                pp = (int*)malloc(num_p*sizeof(int));
                for(j=0, l=0; j<nFaces; j++){
                    if(!hVec_mem_face[j]){
                        pp[l] = hVec[j];
                        l++;
                    }
                }
                index = 0;
                detA = 0.0;
                
                /* While new point is coplanar, choose another point */
                while(detA==0.0){
                    for(j=0;j<d; j++)
                        for(l=0; l<d+1; l++)
                            A[j*(d+1)+l] = points[(faces[k*d+j])*(d+1) + l];
                    for(; j<d+1; j++)
                        for(l=0; l<d+1; l++)
                            A[j*(d+1)+l] = points[pp[index]*(d+1)+l];
                    index++;
                    detA = det_4x4(A);
                }
                
                /* Orient faces so that each point on the original simplex can't see the opposite face */
                if (detA<0.0){
                    /* If orientation is improper, reverse the order to change the volume sign */
                    for(j=0; j<d; j++)
                        face_tmp[j] = faces[k*d+j];
                    for(j=0, l=d-2; j<d-1; j++, l++)
                        faces[k*d+l] = face_tmp[d-j-1];
                    
                    /* Modify the plane coefficients of the properly oriented faces */
                    for(j=0; j<d; j++)
                        cf[k*d+j] = -cf[k*d+j];
                    df[k] = -df[k];
                    for(l=0; l<d; l++)
                        for(j=0; j<d+1; j++)
                            A[l*(d+1)+j] = points[(faces[k*d+l])*(d+1) + j];
                    for(; l<d+1; l++)
                        for(j=0; j<d+1; j++)
                            A[l*(d+1)+j] = points[pp[index]*(d+1)+j];
                }
                free(pp);
            }
            free(horizon);
            free(f0);
            free(nonvisible_faces);
            free(visible);
            free(hVec);
            free(hVec_mem_face);
        }
        if(FUCKED){
            break;
        }
    }
    
    /* output */
    if(FUCKED){
        (*out_faces) = NULL;
        (*nOut_faces) = 0;
    }
    else{
        (*out_faces) = (int*)malloc(nFaces*d*sizeof(int));
        memcpy((*out_faces),faces, nFaces*d*sizeof(int));
        (*nOut_faces) = nFaces;
    }
    
    /* clean-up */
    free(visible_ind);
    free(points_cf);
    free(points_s);
    free(face_s);
    free(gVec);
    free(meanp);
    free(absdist);
    free(reldist);
    free(desReldist);
    free(ind);
    free(span);
    free(points);
    free(faces);
    free(aVec);
    free(cf);
    free(cfi);
    free(df);
    free(p_s);
    free(face_tmp);
    free(fVec);
    free(asfVec);
    free(bVec);
    free(A);
}

void convhull_3d_export_obj
(
    ch_vertex* const vertices,
    const int nVert,
    int* const faces,
    const int nFaces,
    const int keepOnlyUsedVerticesFLAG,
    char* const obj_filename
)
{
    int i, j;
    char path[256] = "\0";
	CV_STRNCPY(path, obj_filename, strlen(obj_filename));
	FILE* obj_file;
#if defined(_MSC_VER) && !defined(_CRT_SECURE_NO_WARNINGS)
	strcat_s(path, ".obj");
	fopen_s(&obj_file, path, "wt");
#else
	obj_file = fopen(strcat(path, ".obj"), "wt");
#endif
    fprintf(obj_file, "o\n");
    CH_FLOAT scale;
    ch_vec3 v1, v2, normal;

    /* export vertices */
    if(keepOnlyUsedVerticesFLAG){
        for (i = 0; i < nFaces; i++)
            for(j=0; j<3; j++)
                fprintf(obj_file, "v %f %f %f\n", vertices[faces[i*3+j]].x,
                        vertices[faces[i*3+j]].y, vertices[faces[i*3+j]].z);
    }
    else {
        for (i = 0; i < nVert; i++)
            fprintf(obj_file, "v %f %f %f\n", vertices[i].x,
                    vertices[i].y, vertices[i].z);
    }
    
    /* export the face normals */
    for (i = 0; i < nFaces; i++){
        /* calculate cross product between v1-v0 and v2-v0 */
        v1 = vertices[faces[i*3+1]];
        v2 = vertices[faces[i*3+2]];
        v1.x -= vertices[faces[i*3]].x;
        v1.y -= vertices[faces[i*3]].y;
        v1.z -= vertices[faces[i*3]].z;
        v2.x -= vertices[faces[i*3]].x;
        v2.y -= vertices[faces[i*3]].y;
        v2.z -= vertices[faces[i*3]].z;
        normal = cross(&v1, &v2);
        
        /* normalise to unit length */
        scale = 1.0/(sqrt(pow(normal.x, 2.0)+pow(normal.y, 2.0)+pow(normal.z, 2.0))+2.23e-9);
        normal.x *= scale;
        normal.y *= scale;
        normal.z *= scale;
        fprintf(obj_file, "vn %f %f %f\n", normal.x, normal.y, normal.z);
    }
    
    /* export the face indices */
    if(keepOnlyUsedVerticesFLAG){
        for (i = 0; i < nFaces; i++){
            /* vertices are in same order as the faces, and normals are in order */
            fprintf(obj_file, "f %u//%u %u//%u %u//%u\n",
                    i*3 + 1, i + 1,
                    i*3+1 + 1, i + 1,
                    i*3+2 + 1, i + 1);
        }
    }
    else {
        /* just normals are in order  */
        for (i = 0; i < nFaces; i++){
            fprintf(obj_file, "f %u//%u %u//%u %u//%u\n",
                    faces[i*3] + 1, i + 1,
                    faces[i*3+1] + 1, i + 1,
                    faces[i*3+2] + 1, i + 1);
        }
    }
    fclose(obj_file);
}

void convhull_3d_export_m
(
    ch_vertex* const vertices,
    const int nVert,
    int* const faces,
    const int nFaces,
    char* const m_filename
)
{
    int i;
	char path[256] = { "\0" }; 
	memcpy(path, m_filename, strlen(m_filename));
	FILE* m_file; 
#if defined(_MSC_VER) && !defined(_CRT_SECURE_NO_WARNINGS)
	CV_STRCAT(path, ".m");
	fopen_s(&m_file, path, "wt");
#else
	m_file = fopen(strcat(path, ".m"), "wt");
#endif
    
    /* save face indices and vertices for verification in matlab: */
    fprintf(m_file, "vertices = [\n");
    for (i = 0; i < nVert; i++)
        fprintf(m_file, "%f, %f, %f;\n", vertices[i].x, vertices[i].y, vertices[i].z);
    fprintf(m_file, "];\n\n\n");
    fprintf(m_file, "faces = [\n");
    for (i = 0; i < nFaces; i++) {
        fprintf(m_file, " %u, %u, %u;\n",
                faces[3*i+0]+1,
                faces[3*i+1]+1,
                faces[3*i+2]+1);
    }
    fprintf(m_file, "];\n\n\n");
    fclose(m_file);
}

void extractVerticesFromObjFile(char* const obj_filename, ch_vertex** out_vertices, int* out_nVert)
{
    FILE* obj_file;
#if defined(_MSC_VER) && !defined(_CRT_SECURE_NO_WARNINGS)
	CV_STRCAT(obj_filename, ".obj");
	fopen_s(&obj_file, obj_filename, "r");
#else
	obj_file = fopen(strcat(obj_filename, ".obj"), "r");
#endif 
    
    /* determine number of vertices */
    unsigned int nVert = 0;
    char line[256];
    while (fgets(line, sizeof(line), obj_file)) {
        char* vexists = strstr(line, "v ");
        if(vexists!=NULL)
            nVert++;
    }
    (*out_nVert) = nVert;
    (*out_vertices) = (ch_vertex*)malloc(nVert*sizeof(ch_vertex));
    
    /* extract the vertices */
    rewind(obj_file);
    int i=0;
    int vertID, prev_char_isDigit, current_char_isDigit;
    char vert_char[256] = { 0 }; 
    while (fgets(line, sizeof(line), obj_file)) {
        char* vexists = strstr(line, "v ");
        if(vexists!=NULL){
            prev_char_isDigit = 0;
            vertID = -1;
            for(size_t j=0; j<strlen(line)-1; j++){
                if(isdigit(line[j])||line[j]=='.'||line[j]=='-'||line[j]=='+'||line[j]=='E'||line[j]=='e'){
                    vert_char[strlen(vert_char)] = line[j];
                    current_char_isDigit = 1;
                }
                else
                    current_char_isDigit = 0;
                if((prev_char_isDigit && !current_char_isDigit) || j ==strlen(line)-2 ){
                    vertID++;
                    if(vertID>4){
                        /* not a valid file */
                        free((*out_vertices));
                        (*out_vertices) = NULL;
                        (*out_nVert) = 0;
                        return;
                    }
                    (*out_vertices)[i].v[vertID] = atof(vert_char);
					memset(vert_char, 0, 256 * sizeof(char)); 
                }
                prev_char_isDigit = current_char_isDigit;
            }
            i++;
        }
    }
}


#endif /* CONVHULL_3D_ENABLE */
