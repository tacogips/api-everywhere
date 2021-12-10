import * as React from "react";
import { useState, useEffect } from "react";
import JSONPretty from "react-json-pretty";

import "react-json-pretty/themes/monikai.css";
import { CopyToClipboard } from "react-copy-to-clipboard";

type Props = {
  apiURL: string | null;
  errorMessage: string | null;
  responseCode: string | null;
  responseBody: string | null;
};

const ResponseArea: React.FC<Props> = ({
  apiURL,
  errorMessage,
  responseCode,
  responseBody,
}) => {
  return (
    <>
      {errorMessage ? (
        <>
          <div role="alert">
            <div className="bg-red-500 text-white font-bold rounded-t px-4 py-2">
              Error
            </div>
            <div className="border border-t-0 border-red-400 rounded-b bg-red-100 px-4 py-3 text-red-700">
              <p>{errorMessage}</p>
            </div>
          </div>
        </>
      ) : (
        <></>
      )}
      {apiURL ? (
        <form className="bg-white shadow-md rounded px-8 pt-6 pb-8 mb-4">
          <h2 className="mb-3 font-bold">Response</h2>
          <label
            className="block text-gray-700 text-sm font-bold mb-2"
            htmlFor="api_url"
          >
            API url
            <CopyToClipboard text={apiURL}>
              <span className="bg-white hover:bg-gray-100 text-gray-600  py-1 px-3 border border-gray-400 rounded shadow ml-3">
                Copy
              </span>
            </CopyToClipboard>
          </label>

          <div id="api_url" className="mb-3">
            <input
              className="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
              id="api_url"
              type="text"
              value={apiURL}
              readOnly={true}
            />
          </div>

          <label
            className="block text-gray-700 text-sm font-bold mb-2"
            htmlFor="response_code"
          >
            response code
          </label>
          <div id="response_code" className="mb-3">
            {responseCode || "-"}
          </div>

          <label
            className="block text-gray-700 text-sm font-bold mb-2"
            htmlFor="response_body"
          >
            response body
            <CopyToClipboard text={JSON.stringify(responseBody)}>
              <span className="bg-white hover:bg-gray-100 text-gray-600  py-1 px-3 border border-gray-400 rounded shadow ml-3">
                Copy
              </span>
            </CopyToClipboard>
          </label>

          <div
            id="response_body"
            className="overflow-auto"
            style={{ maxHeight: "72rem" }}
          >
            <JSONPretty data={responseBody} />
          </div>
        </form>
      ) : (
        <></>
      )}
    </>
  );
};

export default ResponseArea;
